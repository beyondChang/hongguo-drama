// Bridge-only named-export wrapper for the pure window-layout helpers.
const layout = require('./window-layout');
function fitWindowBounds(mode, current, workArea, previous) {
  return layout.fitWindowBounds(mode, current, workArea, previous);
}
function getWindowScale(value) {
  return arguments.length === 0 ? layout.getWindowScale() : layout.getWindowScale(value);
}
module.exports = { fitWindowBounds, getWindowScale };
