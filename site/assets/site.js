const mobileViewport = window.matchMedia('(max-width: 767px)');
const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
const hero = document.querySelector('.hero');
let parallaxQueued = false;
function updateParallax() {
  if (mobileViewport.matches || reducedMotionQuery.matches) {
    document.querySelectorAll('[data-depth]').forEach((element) => { element.style.transform = ''; });
    return;
  }
  if (parallaxQueued) return;
  parallaxQueued = true;
  requestAnimationFrame(() => {
    const scrollY = window.scrollY;
    document.querySelectorAll('[data-depth]').forEach((element) => {
      element.style.transform = `translateY(${scrollY * Number(element.dataset.depth)}px)`;
    });
    parallaxQueued = false;
  });
}
window.addEventListener('scroll', updateParallax, { passive: true });
mobileViewport.addEventListener('change', updateParallax);
reducedMotionQuery.addEventListener('change', updateParallax);

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
  if (!entries.some((entry) => entry.isIntersecting)) return;
  galleryImages.forEach((image) => {
    image.dataset.loaded = 'true';
    setGalleryImage(image);
  });
  observer.disconnect();
}, { rootMargin: '0px 0px -250px 0px' });
galleryLoader.observe(document.querySelector('#gallery'));
document.querySelectorAll('.stars i:not(.static)').forEach((star) => {
  star.style.animationDelay = `${-Math.random() * 6}s`;
});
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
