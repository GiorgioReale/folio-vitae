/**
 * Menu return-path + hash focus.
 *
 * Acceptance (observable):
 * - Menu → /curriculum#section: after navigation, keyboard/AT focus is on the
 *   target section (tabindex=-1); Tab continues from there; focus is visible.
 * - Deep links and in-page hash changes behave the same way.
 */
document.addEventListener("DOMContentLoaded", () => {
  const menuLink = document.querySelector("[data-menu-control]");
  if (menuLink) {
    const returnKey = "folio-menu-return";
    const readReturnPath = () => {
      try {
        return sessionStorage.getItem(returnKey);
      } catch {
        return null;
      }
    };
    const writeReturnPath = (value) => {
      try {
        sessionStorage.setItem(returnKey, value);
      } catch {
        // Storage can be unavailable. The link still has a working href.
      }
    };

    if (menuLink.dataset.menuState === "close") {
      const stored = readReturnPath();
      if (
        stored &&
        stored.startsWith("/") &&
        !stored.startsWith("//") &&
        !stored.includes("://") &&
        !stored.includes("\\")
      ) {
        menuLink.setAttribute("href", stored);
      }
    } else {
      menuLink.addEventListener("click", () => {
        writeReturnPath(
          `${window.location.pathname}${window.location.search}${window.location.hash}`
        );
      });
    }
  }

  focusHashTarget();
});

window.addEventListener("hashchange", () => {
  focusHashTarget();
});

function focusHashTarget() {
  const rawHash = window.location.hash;
  if (!rawHash || rawHash.length < 2) {
    return;
  }

  let id;
  try {
    id = decodeURIComponent(rawHash.slice(1));
  } catch {
    return;
  }

  if (!id) {
    return;
  }

  const target = document.getElementById(id);
  if (!(target instanceof HTMLElement)) {
    return;
  }

  if (!target.hasAttribute("tabindex")) {
    target.setAttribute("tabindex", "-1");
  }

  // Browser already scrolled to the hash; keep that position.
  target.focus({ preventScroll: true });
}
