// @flow strict-local

import {fromNodeId} from './types';
import AdjacencyList, {type SerializedAdjacencyList} from './AdjacencyList';
import type {Edge, Node, NodeId} from './types';
import type {TraversalActions, GraphVisitor} from '@parcel/types';

import assert from 'assert';
import nullthrows from 'nullthrows';

export type NullEdgeType = 1;
export type GraphOpts<TNode, TEdgeType: number = 1> = {|
  nodes?: Map<NodeId, TNode>,
  adjacencyList?: SerializedAdjacencyList<TEdgeType>,
  rootNodeId?: ?NodeId,
  nextNodeId?: ?number,
|};

export type SerializedGraph<TNode, TEdgeType: number = 1> = {|
  nodes: Map<NodeId, TNode>,
  adjacencyList: SerializedAdjacencyList<TEdgeType>,
  rootNodeId: ?NodeId,
  nextNodeId: number,
|};

export type AllEdgeTypes = '@@all_edge_types';
export const ALL_EDGE_TYPES: AllEdgeTypes = '@@all_edge_types';

export default class Graph<TNode: Node, TEdgeType: number = 1> {
  nodes: Map<NodeId, TNode>;
  adjacencyList: AdjacencyList<TEdgeType>;
  rootNodeId: ?NodeId;
  nextNodeId: number = 0;

  constructor(opts: ?GraphOpts<TNode, TEdgeType>) {
    this.nodes = opts?.nodes || new Map();
    this.setRootNodeId(opts?.rootNodeId);
    this.nextNodeId = opts?.nextNodeId ?? 0;

    let adjacencyList = opts?.adjacencyList;
    this.adjacencyList = adjacencyList
      ? AdjacencyList.deserialize(adjacencyList)
      : new AdjacencyList<TEdgeType>();
  }

  setRootNodeId(id: ?NodeId) {
    this.rootNodeId = id;
  }

  static deserialize(
    opts: GraphOpts<TNode, TEdgeType>,
  ): Graph<TNode, TEdgeType> {
    return new this({
      nodes: opts.nodes,
      adjacencyList: opts.adjacencyList,
      rootNodeId: opts.rootNodeId,
      nextNodeId: opts.nextNodeId,
    });
  }

  serialize(): SerializedGraph<TNode, TEdgeType> {
    return {
      nodes: this.nodes,
      adjacencyList: this.adjacencyList.serialize(),
      rootNodeId: this.rootNodeId,
      nextNodeId: this.nextNodeId,
    };
  }

  // Returns an iterator of all edges in the graph. This can be large, so iterating
  // the complete list can be costly in large graphs. Used when merging graphs.
  getAllEdges(): Iterator<Edge<TEdgeType | NullEdgeType>> {
    return this.adjacencyList.getAllEdges();
  }

  addNode(node: TNode): NodeId {
    let id = this.adjacencyList.addNode();
    this.nodes.set(id, node);
    return id;
  }

  hasNode(id: NodeId): boolean {
    return this.nodes.has(id);
  }

  getNode(id: NodeId): ?TNode {
    return this.nodes.get(id);
  }

  addEdge(
    from: NodeId,
    to: NodeId,
    type: TEdgeType | NullEdgeType = 1,
  ): boolean {
    if (Number(type) === 0) {
      throw new Error(`Edge type "${type}" not allowed`);
    }

    if (!this.getNode(from)) {
      throw new Error(`"from" node '${fromNodeId(from)}' not found`);
    }

    if (!this.getNode(to)) {
      throw new Error(`"to" node '${fromNodeId(to)}' not found`);
    }

    // this.outboundEdges.addEdge(from, to, type);
    // this.inboundEdges.addEdge(to, from, type);
    return this.adjacencyList.addEdge(from, to, type);
  }

  hasEdge(
    from: NodeId,
    to: NodeId,
    type?: TEdgeType | NullEdgeType = 1,
  ): boolean {
    // return this.outboundEdges.hasEdge(from, to, type);
    return this.adjacencyList.hasEdge(from, to, type);
  }

  getNodeIdsConnectedTo(
    nodeId: NodeId,
    type: TEdgeType | NullEdgeType | Array<TEdgeType | NullEdgeType> = 1,
  ): Array<NodeId> {
    this._assertHasNodeId(nodeId);

    return [...this.adjacencyList.getNodesConnectedTo(nodeId, type)];
    // let inboundByType = this.inboundEdges.getEdgesByType(nodeId);
    // if (inboundByType == null) {
    //   return [];
    // }

    // let nodes;
    // if (type === ALL_EDGE_TYPES) {
    //   nodes = new Set();
    //   for (let [, typeNodes] of inboundByType) {
    //     for (let node of typeNodes) {
    //       nodes.add(node);
    //     }
    //   }
    // } else if (Array.isArray(type)) {
    //   nodes = new Set();
    //   for (let typeName of type) {
    //     for (let node of inboundByType.get(typeName)?.values() ?? []) {
    //       nodes.add(node);
    //     }
    //   }
    // } else {
    //   nodes = new Set(inboundByType.get(type)?.values() ?? []);
    // }

    // return [...nodes];
  }

  getNodeIdsConnectedFrom(
    nodeId: NodeId,
    type: TEdgeType | NullEdgeType | Array<TEdgeType | NullEdgeType> = 1,
  ): Array<NodeId> {
    this._assertHasNodeId(nodeId);

    return [...this.adjacencyList.getNodesConnectedFrom(nodeId, type)];
    // let outboundByType = this.outboundEdges.getEdgesByType(nodeId);
    // if (outboundByType == null) {
    //   return [];
    // }

    // let nodes;
    // if (type === ALL_EDGE_TYPES) {
    //   nodes = new Set();
    //   for (let [, typeNodes] of outboundByType) {
    //     for (let node of typeNodes) {
    //       nodes.add(node);
    //     }
    //   }
    // } else if (Array.isArray(type)) {
    //   nodes = new Set();
    //   for (let typeName of type) {
    //     for (let node of outboundByType.get(typeName)?.values() ?? []) {
    //       nodes.add(node);
    //     }
    //   }
    // } else {
    //   nodes = new Set(outboundByType.get(type)?.values() ?? []);
    // }

    // return [...nodes];
  }

  // Removes node and any edges coming from or to that node
  removeNode(nodeId: NodeId) {
    this._assertHasNodeId(nodeId);

    for (let {type, from} of [
      ...this.adjacencyList.getInboundEdgesByType(nodeId),
    ]) {
      this.removeEdge(
        from,
        nodeId,
        type,
        // Do not allow orphans to be removed as this node could be one
        // and is already being removed.
        false,
      );
    }
    // for (let from of [
    //   ...this.adjacencyList.getNodesConnectedTo(
    //     nodeId,
    //     // $FlowFixMe
    //     ALL_EDGE_TYPES,
    //   ),
    // ]) {
    //   this.removeEdge(
    //     from,
    //     nodeId,
    //     // $FlowFixMe
    //     ALL_EDGE_TYPES /* any type */,
    //     // Do not allow orphans to be removed as this node could be one
    //     // and is already being removed.
    //     false /* removeOrphans */,
    //   );
    // }

    for (let {type, to} of [
      ...this.adjacencyList.getOutboundEdgesByType(nodeId),
    ]) {
      this.removeEdge(nodeId, to, type);
    }

    // for (let to of [
    //   ...this.adjacencyList.getNodesConnectedFrom(
    //     nodeId,
    //     // $FlowFixMe
    //     ALL_EDGE_TYPES,
    //   ),
    // ]) {
    //   this.removeEdge(nodeId, to);
    // }

    let wasRemoved = this.nodes.delete(nodeId);
    assert(wasRemoved);
  }

  removeEdges(nodeId: NodeId, type: TEdgeType | NullEdgeType = 1) {
    this._assertHasNodeId(nodeId);

    // for (let to of this.outboundEdges.getEdges(nodeId, type)) {
    //   this.removeEdge(nodeId, to, type);
    // }
    for (let to of this.getNodeIdsConnectedFrom(nodeId, type)) {
      this.removeEdge(nodeId, to, type);
    }
  }

  // Removes edge and node the edge is to if the node is orphaned
  removeEdge(
    from: NodeId,
    to: NodeId,
    type: TEdgeType | NullEdgeType = 1,
    removeOrphans: boolean = true,
  ) {
    if (!this.adjacencyList.hasEdge(from, to, type)) {
      throw new Error(
        `Edge from ${fromNodeId(from)} to ${fromNodeId(to)} not found!`,
      );
    }

    this.adjacencyList.removeEdge(from, to, type);
    if (removeOrphans && this.isOrphanedNode(to)) {
      this.removeNode(to);
    }
  }

  isOrphanedNode(nodeId: NodeId): boolean {
    this._assertHasNodeId(nodeId);

    if (this.rootNodeId == null) {
      // If the graph does not have a root, and there are inbound edges,
      // this node should not be considered orphaned.
      // return false;
      // for (let id of this.getNodeIdsConnectedTo(nodeId)) {
      //   if (this.hasNode(id)) return false;
      // }
      if ([...this.adjacencyList.getInboundEdgesByType(nodeId)].length) {
        return false;
      }

      return true;
    }

    // Otherwise, attempt to traverse backwards to the root. If there is a path,
    // then this is not an orphaned node.
    let hasPathToRoot = false;
    // go back to traverseAncestors
    this.traverseAncestors(
      nodeId,
      (ancestorId, _, actions) => {
        if (ancestorId === this.rootNodeId) {
          hasPathToRoot = true;
          actions.stop();
        }
      },
      // $FlowFixMe
      ALL_EDGE_TYPES,
    );

    if (hasPathToRoot) {
      return false;
    }

    return true;
  }

  updateNode(nodeId: NodeId, node: TNode): void {
    this._assertHasNodeId(nodeId);
    this.nodes.set(nodeId, node);
  }

  // TODO: remove because this isn't actually used anywhere?
  // replaceNode(
  //   fromNodeId: NodeId,
  //   toNodeId: NodeId,
  //   type: TEdgeType | NullEdgeType = 0,
  // ): void {
  //   this._assertHasNodeId(fromNodeId);
  //   for (let parent of this.inboundEdges.getEdges(fromNodeId, type)) {
  //     this.addEdge(parent, toNodeId, type);
  //     this.removeEdge(parent, fromNodeId, type);
  //   }
  //   this.removeNode(fromNodeId);
  // }

  // Update a node's downstream nodes making sure to prune any orphaned branches
  replaceNodeIdsConnectedTo(
    fromNodeId: NodeId,
    toNodeIds: $ReadOnlyArray<NodeId>,
    replaceFilter?: null | (NodeId => boolean),
    type?: TEdgeType | NullEdgeType = 1,
  ): void {
    this._assertHasNodeId(fromNodeId);

    let outboundEdges = this.getNodeIdsConnectedFrom(fromNodeId, type);
    let childrenToRemove = new Set(
      replaceFilter
        ? [...outboundEdges].filter(toNodeId => replaceFilter(toNodeId))
        : outboundEdges,
    );
    for (let toNodeId of toNodeIds) {
      childrenToRemove.delete(toNodeId);

      if (!this.hasEdge(fromNodeId, toNodeId, type)) {
        this.addEdge(fromNodeId, toNodeId, type);
      }
    }

    for (let child of childrenToRemove) {
      this.removeEdge(fromNodeId, child, type);
    }
  }

  traverse<TContext>(
    visit: GraphVisitor<NodeId, TContext>,
    startNodeId: ?NodeId,
    type: TEdgeType | NullEdgeType | Array<TEdgeType | NullEdgeType> = 1,
  ): ?TContext {
    return this.dfs({
      visit,
      startNodeId,
      getChildren: nodeId => this.getNodeIdsConnectedFrom(nodeId, type),
    });
  }

  filteredTraverse<TValue, TContext>(
    filter: (NodeId, TraversalActions) => ?TValue,
    visit: GraphVisitor<TValue, TContext>,
    startNodeId: ?NodeId,
    type?: TEdgeType | Array<TEdgeType | NullEdgeType>,
  ): ?TContext {
    return this.traverse(mapVisitor(filter, visit), startNodeId, type);
  }

  traverseAncestors<TContext>(
    startNodeId: ?NodeId,
    visit: GraphVisitor<NodeId, TContext>,
    type: TEdgeType | NullEdgeType | Array<TEdgeType | NullEdgeType> = 1,
  ): ?TContext {
    return this.dfs({
      visit,
      startNodeId,
      getChildren: nodeId => this.getNodeIdsConnectedTo(nodeId, type),
    });
  }

  dfs<TContext>({
    visit,
    startNodeId,
    getChildren,
  }: {|
    visit: GraphVisitor<NodeId, TContext>,
    getChildren(nodeId: NodeId): Array<NodeId>,
    startNodeId?: ?NodeId,
  |}): ?TContext {
    let traversalStartNode = nullthrows(
      startNodeId ?? this.rootNodeId,
      'A start node is required to traverse',
    );
    this._assertHasNodeId(traversalStartNode);

    let visited = new Set<NodeId>();
    let stopped = false;
    let skipped = false;
    let actions: TraversalActions = {
      skipChildren() {
        skipped = true;
      },
      stop() {
        stopped = true;
      },
    };

    let walk = (nodeId, context: ?TContext) => {
      if (!this.hasNode(nodeId)) return;
      visited.add(nodeId);

      skipped = false;
      let enter = typeof visit === 'function' ? visit : visit.enter;
      if (enter) {
        let newContext = enter(nodeId, context, actions);
        if (typeof newContext !== 'undefined') {
          // $FlowFixMe[reassign-const]
          context = newContext;
        }
      }

      if (skipped) {
        return;
      }

      if (stopped) {
        return context;
      }

      for (let child of getChildren(nodeId)) {
        if (visited.has(child)) {
          continue;
        }

        visited.add(child);
        let result = walk(child, context);
        if (stopped) {
          return result;
        }
      }

      if (
        typeof visit !== 'function' &&
        visit.exit &&
        // Make sure the graph still has the node: it may have been removed between enter and exit
        this.hasNode(nodeId)
      ) {
        let newContext = visit.exit(nodeId, context, actions);
        if (typeof newContext !== 'undefined') {
          // $FlowFixMe[reassign-const]
          context = newContext;
        }
      }

      if (skipped) {
        return;
      }

      if (stopped) {
        return context;
      }
    };

    return walk(traversalStartNode);
  }

  bfs(visit: (nodeId: NodeId) => ?boolean): ?NodeId {
    let rootNodeId = nullthrows(
      this.rootNodeId,
      'A root node is required to traverse',
    );

    let queue: Array<NodeId> = [rootNodeId];
    let visited = new Set<NodeId>([rootNodeId]);

    while (queue.length > 0) {
      let node = queue.shift();
      let stop = visit(rootNodeId);
      if (stop === true) {
        return node;
      }

      for (let child of this.getNodeIdsConnectedFrom(node)) {
        if (!visited.has(child)) {
          visited.add(child);
          queue.push(child);
        }
      }
    }

    return null;
  }

  findAncestor(nodeId: NodeId, fn: (nodeId: NodeId) => boolean): ?NodeId {
    let res = null;
    this.traverseAncestors(nodeId, (nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res = nodeId;
        traversal.stop();
      }
    });
    return res;
  }

  findAncestors(
    nodeId: NodeId,
    fn: (nodeId: NodeId) => boolean,
  ): Array<NodeId> {
    let res = [];
    this.traverseAncestors(nodeId, (nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res.push(nodeId);
        traversal.skipChildren();
      }
    });
    return res;
  }

  findDescendant(nodeId: NodeId, fn: (nodeId: NodeId) => boolean): ?NodeId {
    let res = null;
    this.traverse((nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res = nodeId;
        traversal.stop();
      }
    }, nodeId);
    return res;
  }

  findDescendants(
    nodeId: NodeId,
    fn: (nodeId: NodeId) => boolean,
  ): Array<NodeId> {
    let res = [];
    this.traverse((nodeId, ctx, traversal) => {
      if (fn(nodeId)) {
        res.push(nodeId);
        traversal.skipChildren();
      }
    }, nodeId);
    return res;
  }

  _assertHasNodeId(nodeId: NodeId) {
    if (!this.hasNode(nodeId)) {
      throw new Error('Does not have node ' + fromNodeId(nodeId));
    }
  }
}

export function mapVisitor<NodeId, TValue, TContext>(
  filter: (NodeId, TraversalActions) => ?TValue,
  visit: GraphVisitor<TValue, TContext>,
): GraphVisitor<NodeId, TContext> {
  function makeEnter(visit) {
    return function mappedEnter(nodeId, context, actions) {
      let value = filter(nodeId, actions);
      if (value != null) {
        return visit(value, context, actions);
      }
    };
  }

  if (typeof visit === 'function') {
    return makeEnter(visit);
  }

  let mapped = {};
  if (visit.enter != null) {
    mapped.enter = makeEnter(visit.enter);
  }

  if (visit.exit != null) {
    mapped.exit = function mappedExit(nodeId, context, actions) {
      let exit = visit.exit;
      if (!exit) {
        return;
      }

      let value = filter(nodeId, actions);
      if (value != null) {
        return exit(value, context, actions);
      }
    };
  }

  return mapped;
}
