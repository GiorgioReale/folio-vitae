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
    return;
  }

  menuLink.addEventListener("click", () => {
    writeReturnPath(
      `${window.location.pathname}${window.location.search}${window.location.hash}`
    );
  });
});
