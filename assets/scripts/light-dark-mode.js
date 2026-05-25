(() => {
  const currentScript = document.currentScript;
  const spritePrefix =
    currentScript?.dataset.spritePrefix ??
    "/assets/icons/main/website/sprites.svg#";
  const THEME_COOKIE_NAME = "theme";
  const ONE_YEAR_IN_SECONDS = 60 * 60 * 24 * 365;
  const getCookieDomain = () => {
    const host = window.location.hostname;
    if (!host.includes(".")) {
      return null;
    }
    const parts = host.split(".");
    if (parts.length < 2) {
      return null;
    }
    return `.${parts.slice(-2).join(".")}`;
  };
  const setThemeCookie = (value) => {
    const cookieSegments = [
      `${THEME_COOKIE_NAME}=${value}`,
      "Path=/",
      "SameSite=Lax",
      `Max-Age=${ONE_YEAR_IN_SECONDS}`,
    ];
    const cookieDomain = getCookieDomain();
    if (cookieDomain) {
      cookieSegments.push(`Domain=${cookieDomain}`);
    }
    document.cookie = cookieSegments.join(";");
  };
  const getThemeCookie = () => {
    const match = document.cookie
      .split(";")
      .map((cookie) => cookie.trim())
      .find((cookie) => cookie.startsWith(`${THEME_COOKIE_NAME}=`));
    if (!match) {
      return null;
    }
    return match.split("=")[1];
  };
  const getPreferredTheme = () =>
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  const applyTheme = (theme, iconButtonThemeSwitch) => {
    switch (theme) {
      case "dark":
        document.documentElement.setAttribute("data-theme", "dark");
        iconButtonThemeSwitch?.setAttribute(
          "xlink:href",
          `${spritePrefix}moon`
        );
        break;
      case "light":
        document.documentElement.setAttribute("data-theme", "light");
        iconButtonThemeSwitch?.setAttribute("xlink:href", `${spritePrefix}sun`);
        break;
      default: {
        const preferredTheme = getPreferredTheme();
        document.documentElement.setAttribute("data-theme", preferredTheme);
        iconButtonThemeSwitch?.setAttribute(
          "xlink:href",
          `${spritePrefix}moon-auto`
        );
      }
    }
  };
  const getInitialTheme = () => {
    const cookieTheme = getThemeCookie();
    if (cookieTheme === "dark" || cookieTheme === "light") {
      return cookieTheme;
    }
    return "auto";
  };
  let currentTheme = "auto";
  const syncThemeWithCookie = (iconButtonThemeSwitch) => {
    const storedTheme = getThemeCookie();
    if (!storedTheme || storedTheme === currentTheme) {
      return;
    }
    currentTheme = storedTheme;
    applyTheme(currentTheme, iconButtonThemeSwitch);
  };
  document.addEventListener("DOMContentLoaded", () => {
    const buttonThemeSwitch = document.querySelector("#theme-switch");
    const iconButtonThemeSwitch = buttonThemeSwitch?.querySelector("use");
    if (!buttonThemeSwitch || !iconButtonThemeSwitch) {
      return;
    }
    const audioThemeSwitch = new Audio("/assets/audios/light-switch.mp3");
    currentTheme = getInitialTheme();
    if (!getThemeCookie()) {
      setThemeCookie(currentTheme);
    }
    applyTheme(currentTheme, iconButtonThemeSwitch);
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => {
        if (currentTheme === "auto") {
          applyTheme(currentTheme, iconButtonThemeSwitch);
        }
      });
    buttonThemeSwitch.addEventListener("click", (event) => {
      event.preventDefault();
      switch (iconButtonThemeSwitch.getAttribute("xlink:href")) {
        case `${spritePrefix}moon-auto`:
          currentTheme = "light";
          break;
        case `${spritePrefix}sun`:
          currentTheme = "dark";
          break;
        case `${spritePrefix}moon`:
          currentTheme = "auto";
          break;
      }
      setThemeCookie(currentTheme);
      applyTheme(currentTheme, iconButtonThemeSwitch);
      audioThemeSwitch.play();
    });
    const syncInterval = window.setInterval(() => {
      syncThemeWithCookie(iconButtonThemeSwitch);
    }, 2000);
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible") {
        syncThemeWithCookie(iconButtonThemeSwitch);
      }
    });
    window.addEventListener("beforeunload", () => {
      window.clearInterval(syncInterval);
    });
  });
})();
