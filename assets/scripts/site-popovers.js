/**
 * Shared open/close fallback for header popovers (theme, language, menu).
 * Native Invoker Commands + Popover handle the common case; this fills gaps.
 */
(() => {
  const supportsPopover = "popover" in HTMLElement.prototype;
  const supportsInvoker = "commandForElement" in HTMLButtonElement.prototype;

  const isOpen = (menu) => {
    if (!menu) {
      return false;
    }
    if (supportsPopover && typeof menu.matches === "function") {
      return menu.matches(":popover-open");
    }
    return menu.classList.contains("is-open");
  };

  const syncExpanded = (button, menu) => {
    button.setAttribute("aria-expanded", isOpen(menu) ? "true" : "false");
  };

  const closeMenu = (menu) => {
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

  const openMenu = (menu, button) => {
    if (!menu) {
      return;
    }
    if (supportsPopover && typeof menu.showPopover === "function") {
      try {
        menu.showPopover({ source: button });
        return;
      } catch {
        try {
          menu.showPopover();
          return;
        } catch {
          // Fall through to the class-based fallback below.
        }
      }
    }
    menu.hidden = false;
    menu.classList.add("is-open");
  };

  const toggleMenu = (menu, button) => {
    if (isOpen(menu)) {
      closeMenu(menu);
    } else {
      openMenu(menu, button);
    }
  };

  document.addEventListener("DOMContentLoaded", () => {
    const controls = [
      ...document.querySelectorAll('button[command="toggle-popover"][commandfor]'),
    ]
      .map((button) => {
        const menuId = button.getAttribute("commandfor");
        const menu = menuId ? document.getElementById(menuId) : null;
        return menu ? { button, menu } : null;
      })
      .filter(Boolean);

    if (controls.length === 0) {
      return;
    }

    for (const { button, menu } of controls) {
      syncExpanded(button, menu);
      menu.addEventListener("toggle", () => {
        syncExpanded(button, menu);
      });

      if (!supportsInvoker) {
        button.addEventListener("click", (event) => {
          event.preventDefault();
          toggleMenu(menu, button);
          syncExpanded(button, menu);
        });
      }
    }

    if (!supportsPopover) {
      for (const { menu } of controls) {
        menu.hidden = true;
      }

      document.addEventListener("pointerdown", (event) => {
        const path = event.composedPath();
        for (const { button, menu } of controls) {
          if (!isOpen(menu)) {
            continue;
          }
          if (path.includes(menu) || path.includes(button)) {
            continue;
          }
          closeMenu(menu);
          syncExpanded(button, menu);
        }
      });

      document.addEventListener("keydown", (event) => {
        if (event.key !== "Escape") {
          return;
        }
        for (const { button, menu } of controls) {
          if (!isOpen(menu)) {
            continue;
          }
          closeMenu(menu);
          syncExpanded(button, menu);
          button.focus();
        }
      });
    }
  });
})();
