/* AI Motion wire format: sixteen 4-digit hexadecimal rows per 16×16 frame. */
(function (root, factory) {
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.AiMotion = api;
})(typeof window !== 'undefined' ? window : null, function () {
  const GRID_SIZE = 16;
  const WORDS_PER_FRAME = GRID_SIZE;
  const PIXELS_PER_FRAME = GRID_SIZE * GRID_SIZE;
  const TRANSITION_MS = 20;
  const POST_TRANSITION_DELAY_MS = 300;

  function parseSequence(source) {
    const cleaned = String(source)
      .replace(/```[^\n]*\n?/g, ' ')
      .replace(/```/g, ' ')
      .replace(/(^|\n)\s*(?:第\s*\d+\s*帧|frame\s*\d+)\s*[:：]?/gi, '$1');
    const tokens = cleaned.split(/[\s,，;；|]+/).filter(Boolean);
    if (!tokens.length) return { frames: [], error: '输入至少 16 个四位十六进制数。' };

    const words = [];
    for (const token of tokens) {
      if (!/^(?:0[xX])?[0-9a-fA-F]{4}$/.test(token)) {
        return { frames: [], error: `“${token}”不是四位十六进制数。` };
      }
      words.push(Number.parseInt(token.replace(/^0x/i, ''), 16));
    }
    if (words.length % WORDS_PER_FRAME !== 0) {
      return {
        frames: [],
        error: `已有 ${words.length} 个数；每帧需要 16 个，还差 ${WORDS_PER_FRAME - (words.length % WORDS_PER_FRAME)} 个。`,
      };
    }

    const frames = [];
    for (let offset = 0; offset < words.length; offset += WORDS_PER_FRAME) {
      const pixels = [];
      for (const word of words.slice(offset, offset + WORDS_PER_FRAME)) {
        for (let bit = 15; bit >= 0; bit--) pixels.push(Boolean(word & (1 << bit)));
      }
      frames.push(pixels);
    }
    return { frames, error: null };
  }

  function encodeFrame(pixels) {
    if (!Array.isArray(pixels) || pixels.length !== PIXELS_PER_FRAME) {
      throw new Error('一帧必须有 256 个像素。');
    }
    const words = [];
    for (let start = 0; start < PIXELS_PER_FRAME; start += 16) {
      let word = 0;
      for (let bit = 0; bit < 16; bit++) word = (word << 1) | Number(Boolean(pixels[start + bit]));
      words.push(word.toString(16).toUpperCase().padStart(4, '0'));
    }
    return words;
  }

  function encodeSequence(frames) {
    return frames.map((frame) => encodeFrame(frame).join(' ')).join('\n');
  }

  function frameFromRows(rows) {
    if (!Array.isArray(rows) || rows.length !== GRID_SIZE ||
        rows.some((row) => !/^[.#]{16}$/.test(row))) {
      throw new Error('像素图必须是 16 行、每行 16 个 . 或 #。');
    }
    return rows.join('').split('').map((pixel) => pixel === '#');
  }

  return { GRID_SIZE, TRANSITION_MS, POST_TRANSITION_DELAY_MS, parseSequence, encodeFrame, encodeSequence, frameFromRows };
});
