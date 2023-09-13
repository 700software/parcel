// @flow

import * as React from 'react';

import {useAppState} from '../AppState';
import SearchView from './SearchView';
import FocusView from './FocusView';
import DetailView from './DetailView';
import GraphView from './GraphView';

export default function App(): React.Node {
  const [state, dispatch] = useAppState();

  return (
    <div style={{height: '100%', width: '100%'}}>
      <div className="tools tools--left">
        <SearchView />
      </div>

      <pre>
        <code>{JSON.stringify(state, null, 2)}</code>
      </pre>
    </div>
  );
}

// function useEdgeTypes(graph) {
//   return React.useMemo(() => {
//     const types = new Set();
//     if (graph == null) {
//       return [...types];
//     }
//
//     for (let [, edgeMap] of graph.edges) {
//       for (let [type] of edgeMap) {
//         types.add(type);
//       }
//     }
//     return types;
//   }, [graph]);
// }

// function convertGraph({
//   expandedNodeId,
//   focusedEdgeTypes,
//   pinnedNodeIds,
//   graph,
// }) {
//   const shownNodeIds = new Set(pinnedNodeIds);
//
//   const edges = [];
//
//   for (let [sourceId, edgeMap] of graph.edges) {
//     for (let [type, targetIds] of edgeMap) {
//       for (let targetId of targetIds) {
//         if (!focusedEdgeTypes.has(type)) {
//           continue;
//         }
//
//         if (shownNodeIds.has(sourceId) && shownNodeIds.has(targetId)) {
//           edges.push({
//             source: sourceId,
//             target: targetId,
//             type: type ?? undefined,
//             handleTooltipText: type ?? undefined,
//           });
//         } else if (sourceId === expandedNodeId || targetId === expandedNodeId) {
//           edges.push({
//             source: sourceId,
//             target: targetId,
//             type: type ?? undefined,
//             handleTooltipText: type ?? undefined,
//           });
//           shownNodeIds.add(sourceId);
//           shownNodeIds.add(targetId);
//         }
//       }
//     }
//   }
//
//   return {
//     nodes: [...shownNodeIds].map(id => convertNode(id, graph.nodes.get(id))),
//     edges,
//   };
// }
//
// function convertNode(id, node) {
//   let {id: title, type, value} = node;
//   return {
//     id,
//     title,
//     type,
//     value,
//   };
// }
