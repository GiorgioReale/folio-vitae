(() => {
  const currentScript = document.currentScript;
  const spritePrefix =
    currentScript?.dataset.spritePrefix ??
    "/assets/icons/main/website/sprites.svg#";
  const themeAudioSrc =
    currentScript?.dataset.themeAudio ?? "/assets/audios/light-switch.mp3";
  const THEME_COOKIE_NAME = "theme";
  const ONE_YEAR_IN_SECONDS = 60 * 60 * 24 * 365;
  const THEME_VALUES = new Set(["auto", "light", "dark"]);
  const setThemeCookie = (value) => {
    // Host-only cookie: omit Domain so sibling subdomains cannot overwrite theme.
    const cookieSegments = [
      `${THEME_COOKIE_NAME}=${value}`,
      "Path=/",
      "SameSite=Lax",
      `Max-Age=${ONE_YEAR_IN_SECONDS}`,
    ];
    if (window.location.protocol === "https:") {
      cookieSegments.push("Secure");
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
  const THEME_COLOR_LIGHT = "#ffffff";
  const THEME_COLOR_DARK = "#000000";
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
    if (THEME_VALUES.has(cookieTheme)) {
      return cookieTheme;
    }
    return "auto";
  };
  // Keep browser chrome (and PWA UI) aligned with the resolved data-theme,
  // not only prefers-color-scheme — set both media metas so either match wins.
  const syncThemeColor = (resolvedTheme) => {
    const color =
      resolvedTheme === "dark" ? THEME_COLOR_DARK : THEME_COLOR_LIGHT;
    for (const meta of document.querySelectorAll('meta[name="theme-color"]')) {
      meta.setAttribute("content", color);
    }
  };

  // Apply theme as early as possible to avoid a flash of the wrong palette.
  let currentTheme = getInitialTheme();
  const initialResolvedTheme = resolveTheme(currentTheme);
  document.documentElement.setAttribute("data-theme", initialResolvedTheme);
  syncThemeColor(initialResolvedTheme);

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
    const resolvedTheme = resolveTheme(theme);
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
        document.documentElement.setAttribute("data-theme", resolvedTheme);
        setIcon(iconButtonThemeSwitch, "moon-auto");
      }
    }
    syncThemeColor(resolvedTheme);
    describeTheme(theme, button, announce);
  };
  const syncThemeOptions = (theme, options) => {
    for (const option of options) {
      const selected = option.dataset.themeOption === theme;
      option.setAttribute("aria-pressed", selected ? "true" : "false");
    }
  };
  const supportsPopover = "popover" in HTMLElement.prototype;
  const closeThemeMenu = (menu) => {
    if (!menu) {
      return;
    }
    if (supportsPopover && typeof menu.hidePopover === "function") {
      try {
        menu.hidePopover();
        return;
      } catch {
        // Fall through to the class-based fallback below.
      }
    }
    menu.classList.remove("is-open");
    menu.hidden = true;
  };
  document.addEventListener("DOMContentLoaded", () => {
    const buttonThemeSwitch = document.querySelector("#theme-switch");
    const iconButtonThemeSwitch = buttonThemeSwitch?.querySelector("use");
    const themeMenu = document.querySelector("#theme-menu");
    const themeOptions = [
      ...document.querySelectorAll("[data-theme-option]"),
    ];
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
    syncThemeOptions(currentTheme, themeOptions);

    const selectTheme = (theme, announce) => {
      if (!THEME_VALUES.has(theme) || theme === currentTheme) {
        closeThemeMenu(themeMenu);
        return;
      }
      currentTheme = theme;
      setThemeCookie(currentTheme);
      applyTheme(currentTheme, iconButtonThemeSwitch, buttonThemeSwitch, announce);
      syncThemeOptions(currentTheme, themeOptions);
      closeThemeMenu(themeMenu);
      if (announce && !reducedMotion.matches) {
        audioThemeSwitch.play().catch(() => {});
      }
    };

    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => {
        if (currentTheme === "auto") {
          applyTheme(currentTheme, iconButtonThemeSwitch, buttonThemeSwitch, false);
        }
      });

    for (const option of themeOptions) {
      option.addEventListener("click", () => {
        selectTheme(option.dataset.themeOption, true);
      });
    }

    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState !== "visible") {
        return;
      }
      const storedTheme = getThemeCookie();
      if (!storedTheme || storedTheme === currentTheme || !THEME_VALUES.has(storedTheme)) {
        return;
      }
      currentTheme = storedTheme;
      applyTheme(currentTheme, iconButtonThemeSwitch, buttonThemeSwitch, false);
      syncThemeOptions(currentTheme, themeOptions);
    });
  });
})();
