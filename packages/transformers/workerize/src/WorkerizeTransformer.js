import {Transformer} from '@parcel/plugin';
import path from 'path';

import {generateMainCode, generateWorkerCode} from './generate';

export default new Transformer({
  async transform({asset}) {
    let code = await asset.getCode();

    let exports = getExports(code);
    let originalSpecifier = './' + path.basename(asset.filePath);
    let generatedCode;
    if (asset.env.context === 'browser') {
      generatedCode = generateMainCode(originalSpecifier, exports);
      asset.setCode(generatedCode);
      return [asset];
    } else if (asset.env.context === 'web-worker') {
      generatedCode = generateWorkerCode(originalSpecifier);
      asset.setCode(generatedCode);
      let secondAsset = {
        type: 'js',
        code,
        uniqueKey: 'original-worker',
      };
      return [asset, secondAsset];
    }
  },
});

function getExports(code) {
  // match default
  let exports = [...code.matchAll(/^(\s*)export\s+default\s+/m)].length
    ? ['default']
    : [];
  // match named
  exports = exports.concat(
    [
      ...code.matchAll(
        /^(\s*)export\s+((?:async\s*)?function(?:\s*\*)?|const|let|var)(\s+)([a-zA-Z$_][a-zA-Z0-9$_]*)/gm,
      ),
    ].map(arr => arr[4]),
  );

  if (exports.length === 0) {
    throw new Error('Cannot workerize a worker with no exports');
  }

  return exports;
}
