/**
 * Language page navigation (same pattern as the CV menu page).
 *
 * - Opening /{lang}/languages stores the current path for return.
 * - Closing rewrites the header control to that stored path.
 * - Language options are rewritten to the stored path in the chosen language.
 * Without storage, links still go to each language's homepage.
 */
document.addEventListener("DOMContentLoaded", () => {
  const returnKey = "folio-language-return";
  const languageControl = document.querySelector("[data-language-control]");

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

  const pathSuffix = (pathname) => {
    const segments = pathname.split("/").filter(Boolean);
    if (segments.length <= 1) {
      return "/";
    }
    return `/${segments.slice(1).join("/")}`;
  };

  const buildLanguageUrl = (code, suffix) => {
    if (!suffix || suffix === "/") {
      return `/${code}/`;
    }
    return `/${code}${suffix.startsWith("/") ? suffix : `/${suffix}`}`;
  };

  if (languageControl?.dataset.languageState === "open") {
    languageControl.addEventListener("click", () => {
      writeReturnPath(
        `${window.location.pathname}${window.location.search}${window.location.hash}`
      );
    });
  }

  if (languageControl?.dataset.languageState === "close") {
    const stored = readReturnPath();
    if (isSafePath(stored) && !pathSuffix(stored).startsWith("/languages")) {
      languageControl.setAttribute("href", stored);
    }
  }

  const options = document.querySelectorAll("[data-language-option]");
  if (options.length === 0) {
    return;
  }

  const stored = readReturnPath();
  if (!isSafePath(stored) || pathSuffix(stored).startsWith("/languages")) {
    return;
  }

  const suffix = pathSuffix(stored);
  options.forEach((option) => {
    const code = option.getAttribute("hreflang") || option.getAttribute("lang");
    if (!code) {
      return;
    }
    option.setAttribute("href", buildLanguageUrl(code, suffix));
  });
});
