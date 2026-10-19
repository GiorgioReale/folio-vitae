(() => {
  const currentScript = document.currentScript;
  const spritePrefix =
    currentScript?.dataset.spritePrefix ??
    "/assets/icons/main/website/sprites.svg#";
  const themeAudioSrc =
    currentScript?.dataset.themeAudio ?? "/assets/audios/light-switch.mp3";
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
    if (window.location.protocol === "https:") {
      cookieSegments.push("Secure");
    }
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
  const resolveTheme = (theme) => {
    if (theme === "dark" || theme === "light") {
      return theme;
    }
    return getPreferredTheme();
  };
  const getInitialTheme = () => {
    const cookieTheme = getThemeCookie();
    if (cookieTheme === "dark" || cookieTheme === "light" || cookieTheme === "auto") {
      return cookieTheme;
    }
    return "auto";
  };

  // Apply theme as early as possible to avoid a flash of the wrong palette.
  let currentTheme = getInitialTheme();
  document.documentElement.setAttribute("data-theme", resolveTheme(currentTheme));

  const setIcon = (icon, name) => {
    if (!icon) {
      return;
    }
    const value = `${spritePrefix}${name}`;
    icon.setAttribute("href", value);
    icon.setAttribute("xlink:href", value);
  };
  const themeStatus = () => document.querySelector("[data-theme-status]");
  const describeTheme = (theme, button, announce) => {
    if (!button) {
      return;
    }
    const stateLabel =
      theme === "dark"
        ? button.dataset.themeDark
        : theme === "light"
          ? button.dataset.themeLight
          : button.dataset.themeAuto;
    const name = button.querySelector("[data-theme-name]");
    if (name && button.dataset.themeAction && stateLabel) {
      name.textContent = `${button.dataset.themeAction}: ${stateLabel}`;
    }
    if (!announce) {
      return;
    }
    const status = themeStatus();
    if (status && stateLabel) {
      status.textContent = stateLabel;
    }
  };
  const applyTheme = (theme, iconButtonThemeSwitch, button, announce) => {
    switch (theme) {
      case "dark":
        document.documentElement.setAttribute("data-theme", "dark");
        setIcon(iconButtonThemeSwitch, "moon");
        break;
      case "light":
        document.documentElement.setAttribute("data-theme", "light");
        setIcon(iconButtonThemeSwitch, "sun");
        break;
      default: {
        document.documentElement.setAttribute("data-theme", getPreferredTheme());
        setIcon(iconButtonThemeSwitch, "moon-auto");
      }
    }
    describeTheme(theme, button, announce);
  };
  const syncThemeWithCookie = (iconButtonThemeSwitch, button) => {
    const storedTheme = getThemeCookie();
    if (!storedTheme || storedTheme === currentTheme) {
      return;
    }
    currentTheme = storedTheme;
    applyTheme(currentTheme, iconButtonThemeSwitch, button, false);
  };
  document.addEventListener("DOMContentLoaded", () => {
    const buttonThemeSwitch = document.querySelector("#theme-switch");
    const iconButtonThemeSwitch = buttonThemeSwitch?.querySelector("use");
    if (!buttonThemeSwitch || !iconButtonThemeSwitch) {
      return;
    }
    const audioThemeSwitch = new Audio(themeAudioSrc);
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    currentTheme = getInitialTheme();
    if (!getThemeCookie()) {
      setThemeCookie(currentTheme);
    }
    applyTheme(currentTheme, iconButtonThemeSwitch, buttonThemeSwitch, false);
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => {
        if (currentTheme === "auto") {
          applyTheme(currentTheme, iconButtonThemeSwitch, buttonThemeSwitch, false);
        }
      });
    buttonThemeSwitch.addEventListener("click", (event) => {
      event.preventDefault();
      switch (currentTheme) {
        case "auto":
          currentTheme = "light";
          break;
        case "light":
          currentTheme = "dark";
          break;
        default:
          currentTheme = "auto";
      }
      setThemeCookie(currentTheme);
      applyTheme(currentTheme, iconButtonThemeSwitch, buttonThemeSwitch, true);
      if (!reducedMotion.matches) {
        audioThemeSwitch.play().catch(() => {});
      }
    });
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible") {
        syncThemeWithCookie(iconButtonThemeSwitch, buttonThemeSwitch);
      }
    });
  });
})();
