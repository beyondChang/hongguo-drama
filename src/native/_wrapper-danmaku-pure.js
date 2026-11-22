// Bridge-only named-export wrapper for the pure danmaku helpers.
// Network fetch/cache/device-identity functions stay in the L2/L5 shells.
const d = require('./danmaku');
function normalizeColor(value) { return d.normalizeColor(value); }
// The rest parameter is packed into one array argument so the wire can carry it as a Vec.
function normalizeMode(holders) { return d.normalizeMode(...(holders || [])); }
function cleanControl(text) { return d.cleanControl(text); }
function toSeconds(value) { return d.toSeconds(value); }
function parsePage(payload, vid) { return d.parsePage(payload, vid); }
function buildBody(vid, bookId, start, duration) { return d.buildBody(vid, bookId, start, duration); }
module.exports = { normalizeColor, normalizeMode, cleanControl, toSeconds, parsePage, buildBody };
