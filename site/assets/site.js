const mobileViewport = window.matchMedia('(max-width: 767px)');
const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
const hero = document.querySelector('.hero');
const skyLayer = document.querySelector('.sky-layer');
const skyImage = document.querySelector('.sky');
const starField = document.querySelector('.stars');
const meteorField = document.querySelector('.meteors');
const moonFrame = document.querySelector('.moon-frame');
const depthLayers = [...document.querySelectorAll('[data-depth]')];

// Each layer moves down by a share of the scroll, so far layers lag behind near ones.
// Phones get the same effect at a smaller scale.
let parallaxQueued = false;
function updateParallax() {
  if (reducedMotionQuery.matches) {
    depthLayers.forEach((element) => { element.style.transform = ''; });
    return;
  }
  if (parallaxQueued) return;
  parallaxQueued = true;
  requestAnimationFrame(() => {
    const scrollY = Math.min(Math.max(window.scrollY, 0), hero.offsetHeight);
    const scale = mobileViewport.matches ? 0.7 : 1;
    depthLayers.forEach((element) => {
      element.style.transform = `translate3d(0, ${(scrollY * Number(element.dataset.depth) * scale).toFixed(1)}px, 0)`;
    });
    parallaxQueued = false;
  });
}
window.addEventListener('scroll', updateParallax, { passive: true });
mobileViewport.addEventListener('change', updateParallax);
reducedMotionQuery.addEventListener('change', updateParallax);
updateParallax();

// Stars painted into the sky image, as [x%, y%, size] of the 16:9 artwork.
// A glint sits on each one so the painting itself twinkles.
const paintedStars = [
  [69.2, 12.3, 3], [79.1, 12.1, 3], [67.4, 29.1, 2], [29.3, 6.8, 2], [39.5, 13.7, 2], [59.7, 2.6, 2],
  [40.3, 62.1, 2], [65.8, 17.9, 2], [51.4, 85.0, 1], [61.3, 57.3, 2], [50.8, 61.7, 1], [78.6, 3.3, 1],
  [20.6, 29.9, 1], [76.5, 21.0, 1], [63.0, 23.5, 1], [30.6, 12.2, 1], [32.6, 44.1, 1], [22.7, 26.6, 1],
  [78.2, 16.8, 1], [43.6, 56.9, 1], [58.6, 58.0, 1], [35.0, 8.1, 1], [34.8, 30.7, 1], [58.3, 9.0, 1],
  [76.1, 38.8, 1], [82.4, 8.0, 1], [56.2, 33.0, 1], [17.0, 19.7, 1],
];
const glintSizes = [0, 10, 16, 26];
let seed = 2090;
function seededRandom() {
  seed = (seed * 1664525 + 1013904223) % 4294967296;
  return seed / 4294967296;
}
function twinkleTiming(element, minSeconds, maxSeconds) {
  const duration = minSeconds + seededRandom() * (maxSeconds - minSeconds);
  element.style.setProperty('--d', `${duration.toFixed(2)}s`);
  element.style.setProperty('--delay', `${(-seededRandom() * duration).toFixed(2)}s`);
}
const glints = paintedStars.map(([x, y, size]) => {
  const glint = document.createElement('i');
  glint.className = 'glint';
  glint.style.setProperty('--s', `${glintSizes[size]}px`);
  glint.style.setProperty('--lo', '0.08');
  twinkleTiming(glint, 3.5, 7.5);
  glint.dataset.x = x;
  glint.dataset.y = y;
  starField.append(glint);
  return glint;
});
const dotCount = mobileViewport.matches ? 22 : 44;
for (let i = 0; i < dotCount; i += 1) {
  const dot = document.createElement('i');
  dot.className = 'dot';
  dot.style.setProperty('--x', `${(seededRandom() * 100).toFixed(1)}%`);
  dot.style.setProperty('--y', `${(seededRandom() * 62).toFixed(1)}%`);
  dot.style.setProperty('--s', `${(1.4 + seededRandom() * 1.6).toFixed(1)}px`);
  dot.style.setProperty('--lo', (0.15 + seededRandom() * 0.25).toFixed(2));
  twinkleTiming(dot, 2.2, 6);
  starField.append(dot);
}

// Now and then a random small star flares, on top of its steady twinkle.
const smallStars = [...starField.querySelectorAll('.dot')];
function flashStar() {
  const star = smallStars[Math.floor(Math.random() * smallStars.length)];
  star.animate([
    { offset: 0.3, opacity: 1, transform: 'scale(2.6)', boxShadow: '0 0 12px 3px rgba(255,250,235,.9)' },
  ], { duration: 900 + Math.random() * 800, easing: 'ease-out' });
}
(function scheduleFlash() {
  setTimeout(() => {
    if (!hero.classList.contains('motion-paused')) {
      flashStar();
      if (Math.random() < 0.25) setTimeout(flashStar, 150 + Math.random() * 350);
    }
    scheduleFlash();
  }, 700 + Math.random() * 2300);
}());

// Map artwork positions through object-fit: cover and the sky's zoom.
function placeGlints() {
  const width = skyLayer.clientWidth;
  const height = skyLayer.clientHeight;
  const zoom = new DOMMatrixReadOnly(getComputedStyle(skyImage).transform).a;
  const coverScale = Math.max(width / 16, height / 9);
  const artWidth = 16 * coverScale * zoom;
  const artHeight = 9 * coverScale * zoom;
  glints.forEach((glint) => {
    const x = width / 2 + (Number(glint.dataset.x) / 100 - 0.5) * artWidth;
    const y = height / 2 + (Number(glint.dataset.y) / 100 - 0.5) * artHeight;
    glint.hidden = x < 0 || x > width || y < 0 || y > height;
    glint.style.setProperty('--x', `${x.toFixed(1)}px`);
    glint.style.setProperty('--y', `${y.toFixed(1)}px`);
  });
}
placeGlints();
new ResizeObserver(placeGlints).observe(skyLayer);

// Shooting stars fall down and to the right on a 20-40 degree slope, head first,
// away from the title and clear of the moon, at uneven intervals.
// Phones get shorter, rarer ones.
let meteorTimer = 0;
let meteorsRunning = false;
function launchMeteor() {
  const width = skyLayer.clientWidth;
  const height = skyLayer.clientHeight;
  const small = mobileViewport.matches;
  const fieldBox = meteorField.getBoundingClientRect();
  const moonBox = moonFrame.getBoundingClientRect();
  const moonX = moonBox.left - fieldBox.left + moonBox.width / 2;
  const moonY = moonBox.top - fieldBox.top + moonBox.height / 2;
  const clearance = moonBox.width * 0.45 + 30;
  let angle;
  let startX;
  let startY;
  let endX;
  let endY;
  for (let attempt = 0; attempt < 12; attempt += 1) {
    angle = (20 + Math.random() * 20) * Math.PI / 180;
    const distance = (small ? 150 : 300) + Math.random() * (small ? 90 : 260);
    startX = width * (small ? 0.04 + Math.random() * 0.5 : 0.4 + Math.random() * 0.45);
    startY = height * (small ? 0.03 + Math.random() * 0.15 : 0.04 + Math.random() * 0.3);
    endX = startX + Math.cos(angle) * distance;
    endY = startY + Math.sin(angle) * distance;
    // Distance from the moon's centre to the closest point on the path.
    const along = Math.min(1, Math.max(0, ((moonX - startX) * (endX - startX) + (moonY - startY) * (endY - startY)) / (distance * distance)));
    if (Math.hypot(startX + (endX - startX) * along - moonX, startY + (endY - startY) * along - moonY) > clearance) break;
    if (attempt === 11) return;
  }
  const turn = `rotate(${(angle * 180 / Math.PI).toFixed(1)}deg)`;
  const duration = 650 + Math.random() * 650;

  const meteor = document.createElement('div');
  meteor.className = 'meteor';
  meteor.innerHTML = '<span class="tail"></span><span class="head"></span>';
  meteor.style.setProperty('--len', `${Math.round((small ? 70 : 110) + Math.random() * (small ? 50 : 90))}px`);
  meteorField.append(meteor);
  meteor.animate([
    { transform: `translate(${startX}px, ${startY}px) ${turn}`, opacity: 0 },
    { opacity: 1, offset: 0.12 },
    { opacity: 1, offset: 0.75 },
    { transform: `translate(${endX}px, ${endY}px) ${turn}`, opacity: 0 },
  ], { duration, easing: 'cubic-bezier(.35,.1,.75,1)' }).finished.then(() => meteor.remove(), () => meteor.remove());
  meteor.querySelector('.tail').animate([
    { transform: 'scaleX(.1)' },
    { transform: 'scaleX(1)', offset: 0.5 },
    { transform: 'scaleX(.75)' },
  ], { duration });
}
function scheduleMeteor(delay) {
  clearTimeout(meteorTimer);
  meteorTimer = setTimeout(() => {
    launchMeteor();
    const pair = Math.random() < 0.15;
    const gap = mobileViewport.matches ? 6000 + Math.random() * 8000 : 3500 + Math.random() * 7500;
    scheduleMeteor(pair ? 500 + Math.random() * 700 : gap);
  }, delay);
}
function setMeteorsRunning(running) {
  if (running === meteorsRunning) return;
  meteorsRunning = running;
  if (running) {
    scheduleMeteor(1200 + Math.random() * 1500);
  } else {
    clearTimeout(meteorTimer);
    meteorField.getAnimations({ subtree: true }).forEach((animation) => animation.cancel());
  }
}

let heroVisible = true;
const motionToggle = document.querySelector('.motion-toggle');
let userPaused = localStorage.getItem('lunatui-motion-paused') === 'true';
function motionPaused() { return userPaused || reducedMotionQuery.matches; }
function updateMotionToggle() {
  const paused = motionPaused();
  motionToggle.setAttribute('aria-pressed', String(paused));
  motionToggle.disabled = reducedMotionQuery.matches;
  motionToggle.textContent = reducedMotionQuery.matches ? 'Motion reduced' : paused ? 'Play motion' : 'Pause motion';
  document.documentElement.classList.toggle('motion-paused', paused);
  galleryImages.forEach((image) => {
    if (image.dataset.loaded === 'true') setGalleryImage(image);
  });
}

function updateHeroMotion() {
  const paused = !heroVisible || document.hidden || motionPaused();
  hero.classList.toggle('motion-paused', paused);
  setMeteorsRunning(!paused);
}

new IntersectionObserver(([entry]) => {
  heroVisible = entry.isIntersecting;
  updateHeroMotion();
}).observe(hero);
document.addEventListener('visibilitychange', updateHeroMotion);
reducedMotionQuery.addEventListener('change', updateHeroMotion);
reducedMotionQuery.addEventListener('change', updateMotionToggle);
motionToggle.addEventListener('click', () => {
  userPaused = !userPaused;
  localStorage.setItem('lunatui-motion-paused', String(userPaused));
  updateMotionToggle();
  updateHeroMotion();
});

const menuButton = document.querySelector('.menu');
const mainNav = document.querySelector('#main-links');
const mobileMenu = window.matchMedia('(max-width: 767px)');
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

function setMenuOpen(open) {
  menuButton.setAttribute('aria-expanded', String(open));
  menuButton.setAttribute('aria-label', open ? 'Close menu' : 'Open menu');
  mainNav.classList.toggle('open', open);
}

menuButton.addEventListener('click', () => {
  setMenuOpen(menuButton.getAttribute('aria-expanded') !== 'true');
});
mainNav.querySelectorAll('a').forEach((link) => {
  link.addEventListener('click', () => setMenuOpen(false));
});
document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape' && menuButton.getAttribute('aria-expanded') === 'true') {
    setMenuOpen(false);
    menuButton.focus();
  }
});
mobileMenu.addEventListener('change', () => setMenuOpen(false));

const galleryImages = [...document.querySelectorAll('.widget-demo[data-static]')];
function setGalleryImage(image) {
  const source = motionPaused() || !image.dataset.gif ? image.dataset.static : image.dataset.gif;
  if (source && image.getAttribute('src') !== source) image.src = source;
}
const galleryLoader = new IntersectionObserver((entries, observer) => {
  entries.forEach((entry) => {
    if (!entry.isIntersecting) return;
    entry.target.dataset.loaded = 'true';
    setGalleryImage(entry.target);
    observer.unobserve(entry.target);
  });
}, { rootMargin: '200px' });
galleryImages.forEach((image) => galleryLoader.observe(image));
updateMotionToggle();
updateHeroMotion();

const carousel = document.querySelector('#carousel');
const dots = [...document.querySelectorAll('.dots button')];
dots.forEach((button, index) => {
  button.addEventListener('click', () => {
    const cardIndex = Math.round(index * (carousel.children.length - 1) / Math.max(1, dots.length - 1));
    carousel.children[cardIndex].scrollIntoView({
      behavior: reducedMotion.matches ? 'auto' : 'smooth',
      inline: 'center',
      block: 'nearest',
    });
  });
});
let scrollQueued = false;
carousel.addEventListener('scroll', () => {
  if (scrollQueued) return;
  scrollQueued = true;
  requestAnimationFrame(() => {
    const cards = [...carousel.children];
    const center = carousel.getBoundingClientRect().left + carousel.clientWidth / 2;
    const index = cards.reduce((best, card, i) => {
      const distance = Math.abs(card.getBoundingClientRect().left + card.offsetWidth / 2 - center);
      return distance < best.distance ? { index: i, distance } : best;
    }, { index: 0, distance: Infinity }).index;
    const pageIndex = Math.round(index * (dots.length - 1) / Math.max(1, cards.length - 1));
    dots.forEach((dot, i) => {
      if (i === pageIndex) dot.setAttribute('aria-current', 'true');
      else dot.removeAttribute('aria-current');
    });
    scrollQueued = false;
  });
}, { passive: true });

const copyButton = document.querySelector('.copy');
copyButton.addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText('cargo add lunatui');
    copyButton.setAttribute('aria-label', 'Install command copied');
  } catch {
    copyButton.setAttribute('aria-label', 'Install command: cargo add lunatui');
  }
});
