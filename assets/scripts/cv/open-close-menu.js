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

    const isSafePath = (path) => {
      if (typeof path !== "string" || !path.startsWith("/") || path.startsWith("//")) {
        return false;
      }
      if (path.includes("\\") || path.includes("\0")) {
        return false;
      }

      try {
        const url = new URL(path, window.location.origin);
        return (
          url.origin === window.location.origin &&
          url.pathname.startsWith("/") &&
          !url.pathname.startsWith("//")
        );
      } catch {
        return false;
      }
    };

    if (menuLink.dataset.menuState === "close") {
      const stored = readReturnPath();
      if (isSafePath(stored)) {
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
