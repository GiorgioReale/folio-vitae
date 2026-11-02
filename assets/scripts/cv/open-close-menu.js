/**
 * Menu return-path persistence.
 *
 * Hash targets keep tabindex="-1" for the skip link / native fragment
 * activation, but this script does not move focus on page load.
 */
document.addEventListener("DOMContentLoaded", () => {
  const menuLink = document.querySelector("[data-menu-control]");
  if (!menuLink) {
    return;
  }

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
    return;
  }

  menuLink.addEventListener("click", () => {
    writeReturnPath(
      `${window.location.pathname}${window.location.search}${window.location.hash}`
    );
  });
});
