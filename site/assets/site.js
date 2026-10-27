const mobileViewport = window.matchMedia('(max-width: 767px)');
const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
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

const galleryImages = [...document.querySelectorAll('.widget-demo[data-src]')];
const galleryLoader = new IntersectionObserver((entries, observer) => {
  if (!entries.some((entry) => entry.isIntersecting)) return;
  galleryImages.forEach((image) => {
    image.src = image.dataset.src;
    delete image.dataset.src;
  });
  observer.disconnect();
}, { rootMargin: '0px 0px -250px 0px' });
galleryLoader.observe(document.querySelector('#gallery'));

const carousel = document.querySelector('#carousel');
const dots = [...document.querySelectorAll('.dots button')];
dots.forEach((button, index) => {
  button.addEventListener('click', () => {
    carousel.children[index].scrollIntoView({
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
    dots.forEach((dot, i) => {
      if (i === index) dot.setAttribute('aria-current', 'true');
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

const hero = document.querySelector('.hero');
const fastMeteors = new URLSearchParams(location.search).has('fast');
let heroVisible = true;
new IntersectionObserver(([entry]) => { heroVisible = entry.isIntersecting; }).observe(hero);
function meteor() {
  if (!heroVisible || document.hidden || reducedMotionQuery.matches) return;
  const box = hero.getBoundingClientRect();
  const moon = document.querySelector('.moon').getBoundingClientRect();
  const minX = box.width > 900 ? box.width * 0.45 : 0;
  let x;
  let y;
  do {
    x = minX + Math.random() * (box.width - minX - 80);
    y = box.height * (0.04 + Math.random() * 0.31);
  } while (Math.hypot(x - (moon.left - box.left + moon.width / 2), y - (moon.top - box.top + moon.height / 2)) < moon.width * 0.55 + 120);
  const angle = 22 + Math.random() * 14;
  const rad = angle * Math.PI / 180;
  const travel = 260 + Math.random() * 260;
  const el = document.createElement('i');
  el.className = 'meteor';
  el.setAttribute('aria-hidden', 'true');
  el.style.cssText = `left:${x}px;top:${y}px;width:${100 + Math.random() * 120}px`;
  hero.append(el);
  el.animate([
    { opacity: 0, transform: `translate(0,0) rotate(${angle}deg)` },
    { opacity: 1, offset: 0.15 },
    { opacity: 1, offset: 0.6 },
    { opacity: 0, transform: `translate(${Math.cos(rad) * travel}px,${Math.sin(rad) * travel}px) rotate(${angle}deg)` },
  ], { duration: 650 + Math.random() * 450, easing: 'cubic-bezier(.4,0,.9,.6)' })
    .finished.then(() => el.remove(), () => el.remove());
}
window.__meteor = meteor;
setTimeout(meteor, fastMeteors ? 2500 : 6000);
(function next() {
  setTimeout(() => { meteor(); next(); }, fastMeteors ? 3000 + Math.random() * 3000 : 25000 + Math.random() * 25000);
}());

const starEls = [...document.querySelectorAll('.stars i')];
function twinkle() {
  if (!heroVisible || document.hidden || reducedMotionQuery.matches) return;
  const star = starEls[Math.floor(Math.random() * starEls.length)];
  if (!star || star.offsetParent === null) return;
  star.animate([
    { offset: 0.3, opacity: 1, transform: 'scale(2.8)', boxShadow: '0 0 14px 3px rgba(255,255,255,.95)' },
  ], { duration: 900 + Math.random() * 800, easing: 'ease-out' });
}
(function nextTwinkle() {
  setTimeout(() => {
    twinkle();
    if (Math.random() < 0.25) setTimeout(twinkle, 150 + Math.random() * 350);
    nextTwinkle();
  }, 600 + Math.random() * 2200);
}());
