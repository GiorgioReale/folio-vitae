document.addEventListener("DOMContentLoaded", () => {
  const dialog = document.querySelector("dialog.contact-form");

  if (dialog instanceof HTMLDialogElement) {
    if (!("commandForElement" in HTMLButtonElement.prototype)) {
      document.querySelectorAll(`button[commandfor="${dialog.id}"]`).forEach((button) => {
        button.addEventListener("click", () => {
          const command = button.getAttribute("command");
          if (command === "show-modal") {
            dialog.showModal();
          } else if (command === "close") {
            dialog.close();
          }
        });
      });
    }

    if (!("closedBy" in HTMLDialogElement.prototype) && dialog.getAttribute("closedby") === "any") {
      const outside = (event, target) => {
        const rect = target.getBoundingClientRect();
        return (
          event.clientX < rect.left ||
          event.clientX > rect.right ||
          event.clientY < rect.top ||
          event.clientY > rect.bottom
        );
      };
      let pressedOutside = false;
      dialog.addEventListener("pointerdown", (event) => {
        pressedOutside = outside(event, dialog);
      });
      dialog.addEventListener("pointerup", (event) => {
        if (pressedOutside && outside(event, dialog)) {
          dialog.close();
        }
      });
    }

    if (dialog.hasAttribute("data-open-on-load") && !dialog.open) {
      dialog.showModal();
      dialog.querySelector("[autofocus]")?.focus();
    }
  }

  const form = document.querySelector("[data-contact-form]");
  if (!form) {
    return;
  }

  form.noValidate = true;

  const feedback = document.querySelector("[data-contact-feedback]");
  const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  const fields = [
    {
      element: form.querySelector("#full-name"),
      required: form.dataset.msgFullNameRequired,
      invalid: form.dataset.msgFullNameLength,
    },
    {
      element: form.querySelector("#company"),
      required: form.dataset.msgCompanyRequired,
      invalid: form.dataset.msgCompanyLength,
    },
    {
      element: form.querySelector("#email"),
      required: form.dataset.msgEmailRequired,
      invalid: form.dataset.msgEmail,
      email: true,
    },
    {
      element: form.querySelector("#message"),
      required: form.dataset.msgMessageRequired,
      invalid: form.dataset.msgMessageLength,
    },
  ].filter((field) => field.element);

  const messageFor = (field) => {
    const value = field.element.value.trim();
    const min = Number(field.element.getAttribute("minlength")) || 0;
    const max = Number(field.element.getAttribute("maxlength")) || Number.POSITIVE_INFINITY;
    if (!value) {
      return field.required || "";
    }
    if (value.length < min || value.length > max) {
      return field.invalid || "";
    }
    if (field.email && !emailPattern.test(value)) {
      return field.invalid || "";
    }
    return "";
  };

  const describedBy = (field, invalid) => {
    const ids = [`${field.element.id}-hint`];
    if (invalid) {
      ids.push(`${field.element.id}-error`);
    }
    field.element.setAttribute("aria-describedby", ids.join(" "));
  };

  const clearField = (field) => {
    field.element.classList.remove("is-invalid");
    field.element.removeAttribute("aria-invalid");
    const error = document.getElementById(`${field.element.id}-error`);
    if (error) {
      error.hidden = true;
      error.textContent = "";
    }
    describedBy(field, false);
  };

  const markField = (field, message) => {
    field.element.classList.add("is-invalid");
    field.element.setAttribute("aria-invalid", "true");
    const error = document.getElementById(`${field.element.id}-error`);
    if (error) {
      error.hidden = false;
      error.textContent = message;
    }
    describedBy(field, true);
  };

  form.addEventListener("submit", (event) => {
    let firstInvalid = null;
    fields.forEach((field) => {
      field.element.value = field.element.value.trim();
      const message = messageFor(field);
      if (message) {
        markField(field, message);
        firstInvalid ??= field.element;
        return;
      }
      clearField(field);
    });
    if (!firstInvalid) {
      return;
    }
    event.preventDefault();
    if (feedback) {
      feedback.hidden = true;
    }
    firstInvalid.focus();
  });

  fields.forEach((field) => {
    field.element.addEventListener("input", () => {
      if (field.element.getAttribute("aria-invalid") !== "true") {
        return;
      }
      if (messageFor(field)) {
        return;
      }
      clearField(field);
      if (feedback && feedback.dataset.field === field.element.id) {
        feedback.hidden = true;
      }
    });
  });
});
