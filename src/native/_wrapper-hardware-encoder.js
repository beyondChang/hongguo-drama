// Bridge-only named-export wrapper for the pure ffmpeg argument builder.
// pickHardwareEncoder spawns child processes and belongs to the L4 desktop shell.
const { hardwareEncoderArgs } = require('./hardware-encoder');
function buildEncoderArgs(encoder) {
  return hardwareEncoderArgs(encoder);
}
module.exports = { buildEncoderArgs };
