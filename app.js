const { GRID_SIZE, TRANSITION_MS, POST_TRANSITION_DELAY_MS, parseSequence } = window.AiMotion;
const stage = document.querySelector('#stage');
const grid = document.querySelector('#pixel-grid');
const replyText = document.querySelector('#reply-text');
const cells = [];

let frames = [];
let frameIndex = 0;
let timer = null;
let transitionTimer = null;
let publishedId = null;
let displayedPixels = Array(GRID_SIZE * GRID_SIZE).fill(false);

for (let index = 0; index < GRID_SIZE * GRID_SIZE; index++) {
  const cell = document.createElement('div');
  cell.className = 'pixel';
  cell.setAttribute('aria-hidden', 'true');
  grid.appendChild(cell);
  cells.push(cell);
}

function clearTransition() {
  if (transitionTimer !== null) clearTimeout(transitionTimer);
  transitionTimer = null;
  grid.querySelectorAll('.pixel-flight').forEach((pixel) => pixel.remove());
  cells.forEach((cell) => cell.classList.remove('transition-hidden'));
}

function nearestMatches(fromPixels, toPixels) {
  const sources = [];
  const targets = [];
  fromPixels.forEach((on, index) => { if (on) sources.push(index); });
  toPixels.forEach((on, index) => { if (on) targets.push(index); });

  const unusedTargets = new Set(targets);
  const matches = [];
  for (const source of sources) {
    let nearest = -1;
    let nearestDistance = Infinity;
    for (const target of unusedTargets) {
      const dx = (source % GRID_SIZE) - (target % GRID_SIZE);
      const dy = Math.floor(source / GRID_SIZE) - Math.floor(target / GRID_SIZE);
      const distance = dx * dx + dy * dy;
      if (distance < nearestDistance) {
        nearest = target;
        nearestDistance = distance;
      }
    }
    if (nearest !== -1) {
      unusedTargets.delete(nearest);
      matches.push([source, nearest]);
    }
  }

  return matches;
}

function pixelPath(source, target, rects) {
  let row = Math.floor(source / GRID_SIZE);
  let column = source % GRID_SIZE;
  let x = 0;
  let y = 0;
  const path = [];
  const moveTo = (nextRow, nextColumn) => {
    const current = row * GRID_SIZE + column;
    const next = nextRow * GRID_SIZE + nextColumn;
    x += rects[next].left - rects[current].left;
    y += rects[next].top - rects[current].top;
    row = nextRow;
    column = nextColumn;
    path.push({ x, y });
  };

  const targetRow = Math.floor(target / GRID_SIZE);
  const targetColumn = target % GRID_SIZE;
  while (column !== targetColumn) moveTo(row, column + Math.sign(targetColumn - column));
  while (row !== targetRow) moveTo(row + Math.sign(targetRow - row), column);
  return path;
}

function runTransition(flights) {
  const stepCount = Math.max(0, ...flights.map((flight) => flight.path.length));
  if (!stepCount) return 0;

  let step = 0;
  const advance = () => {
    step++;
    flights.forEach((flight) => {
      const position = flight.path[step - 1];
      if (position) flight.element.style.transform = `translate3d(${position.x}px, ${position.y}px, 0)`;
    });
    if (step < stepCount) transitionTimer = setTimeout(advance, TRANSITION_MS);
    else transitionTimer = setTimeout(clearTransition, TRANSITION_MS);
  };
  advance();
  return stepCount * TRANSITION_MS;
}

function render(pixels = frames[frameIndex] || [], transitionFrom = null) {
  clearTransition();
  displayedPixels = pixels.slice();

  const flights = [];
  const hiddenTargets = [];
  let transitionDuration = 0;
  if (transitionFrom) {
    const matches = nearestMatches(transitionFrom, pixels);
    const gridRect = grid.getBoundingClientRect();
    const rects = cells.map((cell) => cell.getBoundingClientRect());
    const fragment = document.createDocumentFragment();

    for (const [source, target] of matches) {
      if (source === target) continue;
      const from = rects[source];
      const flight = document.createElement('div');
      flight.className = 'pixel-flight';
      flight.style.left = `${from.left - gridRect.left}px`;
      flight.style.top = `${from.top - gridRect.top}px`;
      flight.style.width = `${from.width}px`;
      flight.style.height = `${from.height}px`;
      fragment.appendChild(flight);
      hiddenTargets.push(target);
      flights.push({ element: flight, path: pixelPath(source, target, rects) });
    }

    grid.appendChild(fragment);
    hiddenTargets.forEach((index) => cells[index].classList.add('transition-hidden'));
    cells.forEach((cell, index) => cell.classList.toggle('on', Boolean(pixels[index])));

    if (flights.length || hiddenTargets.length) {
      void grid.offsetWidth;
      transitionDuration = runTransition(flights);
    }
  } else {
    cells.forEach((cell, index) => cell.classList.toggle('on', Boolean(pixels[index])));
  }

  grid.setAttribute('aria-label', frames.length
    ? `16×16 像素动画，第 ${frameIndex + 1} 帧，共 ${frames.length} 帧`
    : '空白的 16×16 像素画布');
  return transitionDuration;
}

function stop() {
  if (timer !== null) clearTimeout(timer);
  timer = null;
}

function scheduleNextFrame(delay = POST_TRANSITION_DELAY_MS) {
  if (frames.length < 2) return;
  timer = setTimeout(() => {
    const previousPixels = displayedPixels.slice();
    frameIndex = (frameIndex + 1) % frames.length;
    const transitionDuration = render(frames[frameIndex], previousPixels);
    scheduleNextFrame(transitionDuration + POST_TRANSITION_DELAY_MS);
  }, delay);
}

function setSequence(source) {
  const result = parseSequence(source);
  if (result.error) return result;
  const previousPixels = displayedPixels.slice();
  const hadSequence = frames.length > 0;
  stop();
  frames = result.frames;
  frameIndex = 0;
  const transitionDuration = render(frames[frameIndex], hadSequence ? previousPixels : null);
  scheduleNextFrame(transitionDuration + POST_TRANSITION_DELAY_MS);
  return result;
}

function setReply(text) {
  replyText.textContent = text;
  replyText.hidden = !text;
  stage.classList.toggle('has-text', Boolean(text));
}

function clear() {
  stop();
  frames = [];
  frameIndex = 0;
  publishedId = null;
  setReply('');
  render(Array(GRID_SIZE * GRID_SIZE).fill(false), displayedPixels);
}

async function loadPublishedReply() {
  try {
    const response = await fetch('./current.json', { cache: 'no-store' });
    if (response.status === 404) {
      if (publishedId !== null) clear();
      return;
    }
    if (!response.ok) return;
    const payload = await response.json();
    if (!payload || payload.size !== GRID_SIZE || typeof payload.id !== 'string' || payload.id === publishedId ||
        typeof payload.text !== 'string' || typeof payload.sequence !== 'string') return;
    if (setSequence(payload.sequence).error) return;
    setReply(payload.text);
    publishedId = payload.id;
  } catch {
    // The local server may be temporarily unavailable; keep the last valid frame.
  }
}

render();
loadPublishedReply();
setInterval(loadPublishedReply, 1000);

window.AiMotionPlayer = { setSequence, setReply, clear };
