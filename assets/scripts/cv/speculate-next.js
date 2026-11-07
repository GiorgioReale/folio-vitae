/**
 * Speculative loading for the CV homepage's likely next navigations.
 *
 * Prefetches same-origin curriculum destinations via Speculation Rules
 * when supported, with <link rel="prefetch"> as a progressive fallback.
 * Skipped when the user prefers reduced data (Save-Data / prefers-reduced-data).
 */
(() => {
  const TARGET_SELECTORS = ["a.curriculum-cta"];

  const prefersReducedData = () => {
    if (navigator.connection?.saveData === true) {
      return true;
    }

    try {
      return window.matchMedia("(prefers-reduced-data: reduce)").matches;
    } catch {
      return false;
    }
  };

  const sameOriginHref = (anchor) => {
    const href = anchor.getAttribute("href");
    if (!href || href.startsWith("#")) {
      return null;
    }

    try {
      const url = new URL(href, window.location.origin);
      if (url.origin !== window.location.origin) {
        return null;
      }
      return `${url.pathname}${url.search}${url.hash}`;
    } catch {
      return null;
    }
  };

  const collectTargetHrefs = () => {
    const hrefs = [];
    const seen = new Set();

    for (const selector of TARGET_SELECTORS) {
      document.querySelectorAll(selector).forEach((anchor) => {
        const href = sameOriginHref(anchor);
        if (!href || seen.has(href)) {
          return;
        }
        seen.add(href);
        hrefs.push(href);
      });
    }

    return hrefs;
  };

  const supportsSpeculationRules = () =>
    HTMLScriptElement.supports?.("speculationrules") === true;

  const installSpeculationRules = () => {
    const script = document.createElement("script");
    script.type = "speculationrules";
    script.textContent = JSON.stringify({
      tag: "cv-homepage-next",
      prefetch: [
        {
          where: {
            or: TARGET_SELECTORS.map((selector) => ({
              selector_matches: selector,
            })),
          },
          eagerness: "eager",
        },
      ],
    });
    document.head.appendChild(script);
  };

  const installPrefetchLinks = (hrefs) => {
    for (const href of hrefs) {
      const link = document.createElement("link");
      link.rel = "prefetch";
      link.href = href;
      link.as = "document";
      document.head.appendChild(link);
    }
  };

  const install = () => {
    if (prefersReducedData()) {
      return;
    }

    const hrefs = collectTargetHrefs();
    if (hrefs.length === 0) {
      return;
    }

    if (supportsSpeculationRules()) {
      installSpeculationRules();
      return;
    }

    installPrefetchLinks(hrefs);
  };

  const schedule = (fn) => {
    if ("requestIdleCallback" in window) {
      requestIdleCallback(() => fn(), { timeout: 2000 });
      return;
    }

    window.setTimeout(fn, 1);
  };

  if (document.readyState === "loading") {
    document.addEventListener(
      "DOMContentLoaded",
      () => {
        schedule(install);
      },
      { once: true }
    );
    return;
  }

  schedule(install);
})();
