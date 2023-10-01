// @flow

import type {ContentKey, NodeId} from '@parcel/graph';
import type {Meta, Symbol} from '@parcel/types';
import type {Diagnostic} from '@parcel/diagnostic';
import type {
  AssetNode,
  CommittedAssetId,
  DependencyNode,
  InternalSourceLocation,
  ParcelOptions,
} from './types';
import {type default as AssetGraph} from './AssetGraph';

import invariant from 'assert';
import nullthrows from 'nullthrows';
import {setEqual} from '@parcel/utils';
import logger from '@parcel/logger';
import {md, convertSourceLocationToHighlight} from '@parcel/diagnostic';
import {fromProjectPathRelative, fromProjectPath} from './projectPath';
import {
  Dependency as DbDependency,
  DependencyFlags,
  Asset as DbAsset,
  AssetFlags,
  SymbolFlags,
  readCachedString,
} from '@parcel/rust';

export function propagateSymbols({
  options,
  assetGraph,
  changedAssetsPropagation,
  assetGroupsWithRemovedParents,
  previousErrors,
}: {|
  options: ParcelOptions,
  assetGraph: AssetGraph,
  changedAssetsPropagation: Set<CommittedAssetId>,
  assetGroupsWithRemovedParents: Set<NodeId>,
  previousErrors?: ?Map<NodeId, Array<Diagnostic>>,
|}): Map<NodeId, Array<Diagnostic>> {
  let db = options.db;
  let changedAssets = new Set(
    [...changedAssetsPropagation].map(id =>
      assetGraph.getNodeIdByContentKey(id),
    ),
  );

  // To reorder once at the end
  let changedDeps = new Set<DependencyNode>();

  // For the down traversal, the nodes with `usedSymbolsDownDirty = true` are exactly
  // `changedAssetsPropagation` (= asset and therefore potentially dependencies changed) or the
  // asset children of `assetGroupsWithRemovedParents` (= fewer incoming dependencies causing less
  // used symbols).
  //
  // The up traversal has to consider all nodes that changed in the down traversal
  // (`useSymbolsUpDirtyDown = true`) which are listed in `changedDepsUsedSymbolsUpDirtyDown`
  // (more or less requested symbols) and in `changedAssetsPropagation` (changing an asset might
  // change exports).

  // The dependencies that changed in the down traversal causing an update in the up traversal.
  let changedDepsUsedSymbolsUpDirtyDown = new Set<ContentKey>();

  let starSymbol = db.starSymbol;
  let defaultSymbol = db.defaultSymbol;

  // Propagate the requested symbols down from the root to the leaves
  propagateSymbolsDown(
    assetGraph,
    changedAssets,
    assetGroupsWithRemovedParents,
    (assetNode, incomingDeps, outgoingDeps) => {
      // exportSymbol -> identifier
      let asset = DbAsset.get(db, assetNode.value);
      let assetSymbols = asset.symbols;
      // identifier -> exportSymbol
      let assetSymbolsInverse;
      if (assetSymbols) {
        assetSymbolsInverse = new Map<number, Set<number>>();
        for (let s of assetSymbols) {
          let set = assetSymbolsInverse.get(s.local);

          if (!set) {
            set = new Set();
            assetSymbolsInverse.set(s.local, set);
          }
          set.add(s.exported);
        }
      }
      let hasNamespaceOutgoingDeps = outgoingDeps.some(
        d =>
          DbDependency.get(db, d.value).symbols?.find(
            s => s.exported === starSymbol,
          )?.local === starSymbol,
      );

      // 1) Determine what the incomingDeps requests from the asset
      // ----------------------------------------------------------

      let isEntry = false;
      let addAll = false;

      // Used symbols that are exported or reexported (symbol will be removed again later) by asset.
      assetNode.usedSymbols = new Set();

      // Symbols that have to be namespace reexported by outgoingDeps.
      let namespaceReexportedSymbols = new Set<number>();

      if (incomingDeps.length === 0) {
        // Root in the runtimes Graph
        assetNode.usedSymbols.add(starSymbol);
        namespaceReexportedSymbols.add(starSymbol);
      } else {
        for (let incomingDep of incomingDeps) {
          let dep = DbDependency.get(db, incomingDep.value);
          if (!(dep.flags & DependencyFlags.HAS_SYMBOLS)) {
            if (dep.sourceAssetId == null) {
              // The root dependency on non-library builds
              isEntry = true;
            } else {
              // A regular dependency with cleared symbols
              addAll = true;
            }
            continue;
          }

          for (let exportSymbol of incomingDep.usedSymbolsDown) {
            if (exportSymbol === starSymbol) {
              assetNode.usedSymbols.add(starSymbol);
              namespaceReexportedSymbols.add(starSymbol);
            }
            if (
              !assetSymbols ||
              assetSymbols.some(s => s.exported === exportSymbol) ||
              assetSymbols.some(s => s.exported === starSymbol)
            ) {
              // An own symbol or a non-namespace reexport
              assetNode.usedSymbols.add(exportSymbol);
            }
            // A namespace reexport
            // (but only if we actually have namespace-exporting outgoing dependencies,
            // This usually happens with a reexporting asset with many namespace exports which means that
            // we cannot match up the correct asset with the used symbol at this level.)
            else if (
              hasNamespaceOutgoingDeps &&
              exportSymbol !== defaultSymbol
            ) {
              namespaceReexportedSymbols.add(exportSymbol);
            }
          }
        }
      }

      // Incomding dependency with cleared symbols, add everything
      if (addAll) {
        for (let sym of assetSymbols) {
          assetNode.usedSymbols.add(sym.exported);
        }
      }

      // 2) Distribute the symbols to the outgoing dependencies
      // ----------------------------------------------------------
      for (let dep of outgoingDeps) {
        let depUsedSymbolsDownOld = dep.usedSymbolsDown;
        let depUsedSymbolsDown = new Set<number>();
        dep.usedSymbolsDown = depUsedSymbolsDown;
        if (
          asset.flags & AssetFlags.SIDE_EFFECTS ||
          // Incoming dependency with cleared symbols
          addAll ||
          // For entries, we still need to add dep.value.symbols of the entry (which are "used" but not according to the symbols data)
          isEntry ||
          // If not a single symbol is used, we can say the entire subgraph is not used.
          // This is e.g. needed when some symbol is imported and then used for a export which isn't used (= "semi-weak" reexport)
          //    index.js:     `import {bar} from "./lib"; ...`
          //    lib/index.js: `export * from "./foo.js"; export * from "./bar.js";`
          //    lib/foo.js:   `import { data } from "./bar.js"; export const foo = data + " esm2";`
          assetNode.usedSymbols.size > 0 ||
          namespaceReexportedSymbols.size > 0
        ) {
          let depSymbols = DbDependency.get(db, dep.value).symbols;
          if (!depSymbols) continue;

          if (
            depSymbols.find(s => s.exported === starSymbol)?.local ===
            starSymbol
          ) {
            if (addAll) {
              depUsedSymbolsDown.add(starSymbol);
            } else {
              for (let s of namespaceReexportedSymbols) {
                // We need to propagate the namespaceReexportedSymbols to all namespace dependencies (= even wrong ones because we don't know yet)
                depUsedSymbolsDown.add(s);
              }
            }
          }

          for (let {exported: symbol, local, flags} of depSymbols) {
            // Was already handled above
            if (local === starSymbol) continue;

            if (!assetSymbolsInverse || !(flags & SymbolFlags.IS_WEAK)) {
              // Bailout or non-weak symbol (= used in the asset itself = not a reexport)
              depUsedSymbolsDown.add(symbol);
            } else {
              let reexportedExportSymbols = assetSymbolsInverse.get(local);
              if (reexportedExportSymbols == null) {
                // not reexported = used in asset itself
                depUsedSymbolsDown.add(symbol);
              } else if (assetNode.usedSymbols.has(starSymbol)) {
                // we need everything
                depUsedSymbolsDown.add(symbol);

                [...reexportedExportSymbols].forEach(s =>
                  assetNode.usedSymbols.delete(s),
                );
              } else {
                let usedReexportedExportSymbols = [
                  ...reexportedExportSymbols,
                ].filter(s => assetNode.usedSymbols.has(s));
                if (usedReexportedExportSymbols.length > 0) {
                  // The symbol is indeed a reexport, so it's not used from the asset itself
                  depUsedSymbolsDown.add(symbol);

                  usedReexportedExportSymbols.forEach(s =>
                    assetNode.usedSymbols.delete(s),
                  );
                }
              }
            }
          }
        } else {
          depUsedSymbolsDown.clear();
        }
        if (!setEqual(depUsedSymbolsDownOld, depUsedSymbolsDown)) {
          dep.usedSymbolsDownDirty = true;
          dep.usedSymbolsUpDirtyDown = true;
          changedDepsUsedSymbolsUpDirtyDown.add(dep.id);
        }
        if (dep.usedSymbolsUpDirtyDown) {
          // Set on node creation
          changedDepsUsedSymbolsUpDirtyDown.add(dep.id);
        }
      }
    },
  );

  const logFallbackNamespaceInsertion = (
    assetNode,
    symbol: number,
    depNode1,
    depNode2,
  ) => {
    if (options.logLevel === 'verbose') {
      logger.warn({
        message: `${fromProjectPathRelative(
          DbAsset.get(db, assetNode.value).filePath,
        )} reexports "${readCachedString(db,
          symbol,
        )}", which could be resolved either to the dependency "${
          DbDependency.get(db, depNode1.value).specifier
        }" or "${
          DbDependency.get(db, depNode2.value).specifier
        }" at runtime. Adding a namespace object to fall back on.`,
        origin: '@parcel/core',
      });
    }
  };

  // Because namespace reexports introduce ambiguity, go up the graph from the leaves to the
  // root and remove requested symbols that aren't actually exported
  let errors = propagateSymbolsUp(
    assetGraph,
    changedAssets,
    changedDepsUsedSymbolsUpDirtyDown,
    previousErrors,
    (assetNode, incomingDeps, outgoingDeps) => {
      let asset = DbAsset.get(db, assetNode.value);
      let assetSymbols = asset.symbols;

      let assetSymbolsInverse = null;
      if (assetSymbols) {
        assetSymbolsInverse = new Map<number, Set<number>>();
        for (let s of assetSymbols) {
          let set = assetSymbolsInverse.get(s.local);
          if (!set) {
            set = new Set();
            assetSymbolsInverse.set(s.local, set);
          }
          set.add(s.exported);
        }
      }

      // the symbols that are reexported (not used in `asset`) -> asset they resolved to
      let reexportedSymbols = new Map<
        number,
        ?{|asset: ContentKey, symbol: ?number|},
      >();
      // the symbols that are reexported (not used in `asset`) -> the corresponding outgoingDep(s)
      // To generate the diagnostic when there are multiple dependencies with non-statically
      // analyzable exports
      let reexportedSymbolsSource = new Map<number, DependencyNode>();
      for (let outgoingDep of outgoingDeps) {
        let outgoingDepSymbols = DbDependency.get(db, outgoingDep.value).symbols;
        if (!outgoingDepSymbols) continue;

        let isExcluded =
          assetGraph.getNodeIdsConnectedFrom(
            assetGraph.getNodeIdByContentKey(outgoingDep.id),
          ).length === 0;
        // excluded, assume everything that is requested exists
        if (isExcluded) {
          outgoingDep.usedSymbolsDown.forEach((_, s) =>
            outgoingDep.usedSymbolsUp.set(s, null),
          );
        }

        if (
          outgoingDepSymbols.find(s => s.exported === starSymbol)?.local ===
          starSymbol
        ) {
          outgoingDep.usedSymbolsUp.forEach((sResolved, s) => {
            if (s === defaultSymbol) {
              return;
            }

            // If the symbol could come from multiple assets at runtime, assetNode's
            // namespace will be needed at runtime to perform the lookup on.
            if (reexportedSymbols.has(s)) {
              if (!assetNode.usedSymbols.has(starSymbol)) {
                logFallbackNamespaceInsertion(
                  assetNode,
                  s,
                  nullthrows(reexportedSymbolsSource.get(s)),
                  outgoingDep,
                );
              }
              assetNode.usedSymbols.add(starSymbol);
              reexportedSymbols.set(s, {asset: assetNode.id, symbol: s});
            } else {
              reexportedSymbols.set(s, sResolved);
              reexportedSymbolsSource.set(s, outgoingDep);
            }
          });
        }

        for (let [s, sResolved] of outgoingDep.usedSymbolsUp) {
          if (!outgoingDep.usedSymbolsDown.has(s)) {
            // usedSymbolsDown is a superset of usedSymbolsUp
            continue;
          }

          let local = outgoingDepSymbols.find(sym => sym.exported === s)?.local;

          if (local == null) {
            // Caused by '*' => '*', already handled
            continue;
          }

          let reexported = assetSymbolsInverse?.get(local);
          if (reexported != null) {
            reexported.forEach(s => {
              // see same code above
              if (reexportedSymbols.has(s)) {
                if (!assetNode.usedSymbols.has(starSymbol)) {
                  logFallbackNamespaceInsertion(
                    assetNode,
                    s,
                    nullthrows(reexportedSymbolsSource.get(s)),
                    outgoingDep,
                  );
                }
                assetNode.usedSymbols.add(starSymbol);
                reexportedSymbols.set(s, {asset: assetNode.id, symbol: s});
              } else {
                reexportedSymbols.set(s, sResolved);
                reexportedSymbolsSource.set(s, outgoingDep);
              }
            });
          }
        }
      }

      let errors: Array<Diagnostic> = [];

      function usedSymbolsUpAmbiguous(old, current, s, value) {
        if (old.has(s)) {
          let valueOld = old.get(s);
          if (
            valueOld !== value &&
            !(
              valueOld?.asset === value.asset &&
              valueOld?.symbol === value.symbol
            )
          ) {
            // The dependency points to multiple assets (via an asset group).
            current.set(s, undefined);
            return;
          }
        }
        current.set(s, value);
      }

      for (let incomingDep of incomingDeps) {
        let dep = DbDependency.get(db, incomingDep.value);
        let incomingDepUsedSymbolsUpOld = incomingDep.usedSymbolsUp;
        incomingDep.usedSymbolsUp = new Map();
        let incomingDepSymbols = dep.symbols;
        if (!incomingDepSymbols) continue;

        let hasNamespaceReexport =
          incomingDepSymbols.find(s => s.exported === starSymbol)?.local ===
          starSymbol;
        for (let s of incomingDep.usedSymbolsDown) {
          if (
            assetSymbols == null || // Assume everything could be provided if symbols are cleared
            asset.bundleBehavior === 'isolated' ||
            asset.bundleBehavior === 'inline' ||
            s === starSymbol ||
            assetNode.usedSymbols.has(s)
          ) {
            usedSymbolsUpAmbiguous(
              incomingDepUsedSymbolsUpOld,
              incomingDep.usedSymbolsUp,
              s,
              {
                asset: assetNode.id,
                symbol: s,
              },
            );
          } else if (reexportedSymbols.has(s)) {
            let reexport = reexportedSymbols.get(s);
            let v =
              // Forward a reexport only if the current asset is side-effect free and not external
              !(asset.flags & AssetFlags.SIDE_EFFECTS) && reexport != null
                ? reexport
                : {
                    asset: assetNode.id,
                    symbol: s,
                  };
            usedSymbolsUpAmbiguous(
              incomingDepUsedSymbolsUpOld,
              incomingDep.usedSymbolsUp,
              s,
              v,
            );
          } else if (!hasNamespaceReexport) {
            let loc = dep.symbols?.find(sym => sym.exported === s)?.loc;
            let [resolutionNodeId] = assetGraph.getNodeIdsConnectedFrom(
              assetGraph.getNodeIdByContentKey(incomingDep.id),
            );
            let resolution = nullthrows(assetGraph.getNode(resolutionNodeId));
            invariant(
              resolution &&
                (resolution.type === 'asset_group' ||
                  resolution.type === 'asset'),
            );

            let incomingDepValue = DbDependency.get(db, incomingDep.value);
            let sourceAsset =
              incomingDepValue.sourceAssetId != null
                ? DbAsset.get(db, incomingDepValue.sourceAssetId)
                : null;
            errors.push({
              message: md`${fromProjectPathRelative(
                (resolution.type === 'asset'
                  ? DbAsset.get(db, resolution.value)
                  : resolution.value
                ).filePath,
              )} does not export '${readCachedString(db, s)}'`,
              origin: '@parcel/core',
              codeFrames: loc
                ? [
                    {
                      filePath:
                        fromProjectPath(options.projectRoot, loc?.filePath) ??
                        undefined,
                      language: sourceAsset?.assetType,
                      codeHighlights: [convertSourceLocationToHighlight(loc)],
                    },
                  ]
                : undefined,
            });
          }
        }

        if (!equalMap(incomingDepUsedSymbolsUpOld, incomingDep.usedSymbolsUp)) {
          changedDeps.add(incomingDep);
          incomingDep.usedSymbolsUpDirtyUp = true;
        }

        incomingDep.excluded = false;
        if (dep.flags & DependencyFlags.HAS_SYMBOLS && incomingDep.usedSymbolsUp.size === 0) {
          let assetGroups = assetGraph.getNodeIdsConnectedFrom(
            assetGraph.getNodeIdByContentKey(incomingDep.id),
          );
          if (assetGroups.length === 1) {
            let [assetGroupId] = assetGroups;
            let assetGroup = nullthrows(assetGraph.getNode(assetGroupId));
            if (
              assetGroup.type === 'asset_group' &&
              assetGroup.value.sideEffects === false
            ) {
              incomingDep.excluded = true;
            }
          } else {
            invariant(assetGroups.length === 0);
          }
        }
      }
      return errors;
    },
  );

  // Sort usedSymbolsUp so they are a consistent order across builds.
  // This ensures a consistent ordering of these symbols when packaging.
  // See https://github.com/parcel-bundler/parcel/pull/8212
  for (let dep of changedDeps) {
    dep.usedSymbolsUp = new Map(
      [...dep.usedSymbolsUp].sort(([a], [b]) => a - b),
    );
  }

  return errors;
}

function propagateSymbolsDown(
  assetGraph: AssetGraph,
  changedAssets: Set<NodeId>,
  assetGroupsWithRemovedParents: Set<NodeId>,
  visit: (
    assetNode: AssetNode,
    incoming: $ReadOnlyArray<DependencyNode>,
    outgoing: $ReadOnlyArray<DependencyNode>,
  ) => void,
) {
  if (changedAssets.size === 0 && assetGroupsWithRemovedParents.size === 0) {
    return;
  }

  // We care about changed assets and their changed dependencies. So start with the first changed
  // asset or dependency and continue while the symbols change. If the queue becomes empty,
  // continue with the next unvisited changed asset.
  //
  // In the end, nodes, which are neither listed in changedAssets nor in
  // assetGroupsWithRemovedParents nor reached via a dirty flag, don't have to be visited at all.
  //
  // In the worst case, some nodes have to be revisited because we don't want to sort the assets
  // into topological order. For example in a diamond graph where the join point is visited twice
  // via each parent (the numbers signifiying the order of re/visiting, `...` being unvisited).
  // However, this only continues as long as there are changes in the used symbols that influence
  // child nodes.
  //
  //             |
  //            ...
  //          /     \
  //          1     4
  //          \     /
  //            2+5
  //             |
  //            3+6
  //             |
  //            ...
  //             |
  //

  let unreachedAssets = new Set([
    ...changedAssets,
    ...assetGroupsWithRemovedParents,
  ]);
  let queue = new Set([setPop(unreachedAssets)]);

  while (queue.size > 0) {
    let queuedNodeId = setPop(queue);
    unreachedAssets.delete(queuedNodeId);

    let outgoing = assetGraph.getNodeIdsConnectedFrom(queuedNodeId);
    let node = nullthrows(assetGraph.getNode(queuedNodeId));

    let wasNodeDirty = false;
    if (node.type === 'dependency' || node.type === 'asset_group') {
      wasNodeDirty = node.usedSymbolsDownDirty;
      node.usedSymbolsDownDirty = false;
    } else if (node.type === 'asset' && node.usedSymbolsDownDirty) {
      visit(
        node,
        assetGraph.getIncomingDependencies(node.value).map(d => {
          let dep = assetGraph.getNodeByContentKey(d);
          invariant(dep && dep.type === 'dependency');
          return dep;
        }),
        outgoing.map(dep => {
          let depNode = nullthrows(assetGraph.getNode(dep));
          invariant(depNode.type === 'dependency');
          return depNode;
        }),
      );
      node.usedSymbolsDownDirty = false;
    }

    for (let child of outgoing) {
      let childNode = nullthrows(assetGraph.getNode(child));
      let childDirty = false;
      if (
        (childNode.type === 'asset' || childNode.type === 'asset_group') &&
        wasNodeDirty
      ) {
        childNode.usedSymbolsDownDirty = true;
        childDirty = true;
      } else if (childNode.type === 'dependency') {
        childDirty = childNode.usedSymbolsDownDirty;
      }
      if (childDirty) {
        queue.add(child);
      }
    }

    if (queue.size === 0 && unreachedAssets.size > 0) {
      queue.add(setPop(unreachedAssets));
    }
  }
}

function propagateSymbolsUp(
  assetGraph: AssetGraph,
  changedAssets: Set<NodeId>,
  changedDepsUsedSymbolsUpDirtyDown: Set<ContentKey>,
  previousErrors: ?Map<NodeId, Array<Diagnostic>>,
  visit: (
    assetNode: AssetNode,
    incoming: $ReadOnlyArray<DependencyNode>,
    outgoing: $ReadOnlyArray<DependencyNode>,
  ) => Array<Diagnostic>,
): Map<NodeId, Array<Diagnostic>> {
  // For graphs in general (so with cyclic dependencies), some nodes will have to be revisited. So
  // run a regular queue-based BFS for anything that's still dirty.
  //
  // (Previously, there was first a recursive post-order DFS, with the idea that all children of a
  // node should be processed first. With a tree, this would result in a minimal amount of work by
  // processing every asset exactly once and then the remaining cycles would have been handled
  // with the loop. This was slightly faster for initial builds but had O(project) instead of
  // O(changes).)

  let errors: Map<NodeId, Array<Diagnostic>> = previousErrors
    ? // Some nodes might have been removed since the last build
      new Map([...previousErrors].filter(([n]) => assetGraph.hasNode(n)))
    : new Map();

  let changedDepsUsedSymbolsUpDirtyDownAssets = new Set([
    ...[...changedDepsUsedSymbolsUpDirtyDown]
      .reverse()
      .flatMap(id => getDependencyResolution(assetGraph, id)),
    ...changedAssets,
  ]);

  // Do a more efficient full traversal (less recomputations) if more than half of the assets
  // changed.
  let runFullPass =
    // If there are n nodes in the graph, then the asset count is approximately
    // n/6 (for every asset, there are ~4 dependencies and ~1 asset_group).
    assetGraph.nodes.length * (1 / 6) * 0.5 <
    changedDepsUsedSymbolsUpDirtyDownAssets.size;

  let dirtyDeps;
  if (runFullPass) {
    dirtyDeps = new Set<NodeId>();
    let rootNodeId = nullthrows(
      assetGraph.rootNodeId,
      'A root node is required to traverse',
    );
    let visited = new Set([rootNodeId]);
    const walk = (nodeId: NodeId) => {
      let node = nullthrows(assetGraph.getNode(nodeId));
      let outgoing = assetGraph.getNodeIdsConnectedFrom(nodeId);
      for (let childId of outgoing) {
        if (!visited.has(childId)) {
          visited.add(childId);
          walk(childId);
          let child = nullthrows(assetGraph.getNode(childId));
          if (node.type === 'asset') {
            invariant(child.type === 'dependency');
            if (child.usedSymbolsUpDirtyUp) {
              node.usedSymbolsUpDirty = true;
              child.usedSymbolsUpDirtyUp = false;
            }
          }
        }
      }

      if (node.type === 'asset') {
        let incoming = assetGraph.getIncomingDependencies(node.value).map(d => {
          let n = assetGraph.getNodeByContentKey(d);
          invariant(n && n.type === 'dependency');
          return n;
        });
        for (let dep of incoming) {
          if (dep.usedSymbolsUpDirtyDown) {
            dep.usedSymbolsUpDirtyDown = false;
            node.usedSymbolsUpDirty = true;
          }
        }
        if (node.usedSymbolsUpDirty) {
          let e = visit(
            node,
            incoming,
            outgoing.map(depNodeId => {
              let depNode = nullthrows(assetGraph.getNode(depNodeId));
              invariant(depNode.type === 'dependency');
              return depNode;
            }),
          );
          if (e.length > 0) {
            node.usedSymbolsUpDirty = true;
            errors.set(nodeId, e);
          } else {
            node.usedSymbolsUpDirty = false;
            errors.delete(nodeId);
          }
        }
      } else {
        if (node.type === 'dependency') {
          if (node.usedSymbolsUpDirtyUp) {
            dirtyDeps.add(nodeId);
          } else {
            dirtyDeps.delete(nodeId);
          }
        }
      }
    };
    walk(rootNodeId);
  }

  let queue = dirtyDeps ?? changedDepsUsedSymbolsUpDirtyDownAssets;
  while (queue.size > 0) {
    let queuedNodeId = setPop(queue);
    let node = nullthrows(assetGraph.getNode(queuedNodeId));
    if (node.type === 'asset') {
      let incoming = assetGraph.getIncomingDependencies(node.value).map(dep => {
        let depNode = assetGraph.getNodeByContentKey(dep);
        invariant(depNode && depNode.type === 'dependency');
        return depNode;
      });
      for (let dep of incoming) {
        if (dep.usedSymbolsUpDirtyDown) {
          dep.usedSymbolsUpDirtyDown = false;
          node.usedSymbolsUpDirty = true;
        }
      }
      let outgoing = assetGraph
        .getNodeIdsConnectedFrom(queuedNodeId)
        .map(depNodeId => {
          let depNode = nullthrows(assetGraph.getNode(depNodeId));
          invariant(depNode.type === 'dependency');
          return depNode;
        });
      for (let dep of outgoing) {
        if (dep.usedSymbolsUpDirtyUp) {
          node.usedSymbolsUpDirty = true;
          dep.usedSymbolsUpDirtyUp = false;
        }
      }

      if (node.usedSymbolsUpDirty) {
        let e = visit(node, incoming, outgoing);
        if (e.length > 0) {
          node.usedSymbolsUpDirty = true;
          errors.set(queuedNodeId, e);
        } else {
          node.usedSymbolsUpDirty = false;
          errors.delete(queuedNodeId);
        }
      }

      for (let i of incoming) {
        if (i.usedSymbolsUpDirtyUp) {
          queue.add(assetGraph.getNodeIdByContentKey(i.id));
        }
      }
    } else {
      let connectedNodes = assetGraph.getNodeIdsConnectedTo(queuedNodeId);
      if (connectedNodes.length > 0) {
        queue.add(...connectedNodes);
      }
    }
  }

  return errors;
}

function getDependencyResolution(
  graph: AssetGraph,
  depId: ContentKey,
): Array<NodeId> {
  let depNodeId = graph.getNodeIdByContentKey(depId);
  let connected = graph.getNodeIdsConnectedFrom(depNodeId);
  invariant(connected.length <= 1);
  let child = connected[0];
  if (child) {
    let childNode = nullthrows(graph.getNode(child));
    if (childNode.type === 'asset_group') {
      return graph.getNodeIdsConnectedFrom(child);
    } else {
      return [child];
    }
  }
  return [];
}

function equalMap<K>(
  a: $ReadOnlyMap<K, ?{|asset: ContentKey, symbol: ?number|}>,
  b: $ReadOnlyMap<K, ?{|asset: ContentKey, symbol: ?number|}>,
) {
  if (a.size !== b.size) return false;
  for (let [k, v] of a) {
    if (!b.has(k)) return false;
    let vB = b.get(k);
    if (vB?.asset !== v?.asset || vB?.symbol !== v?.symbol) return false;
  }
  return true;
}

function setPop<T>(set: Set<T>): T {
  let v = nullthrows(set.values().next().value);
  set.delete(v);
  return v;
}
