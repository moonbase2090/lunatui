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
