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

  const feedback = document.querySelector("[data-contact-feedback]");
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
    },
    {
      element: form.querySelector("#message"),
      required: form.dataset.msgMessageRequired,
      invalid: form.dataset.msgMessageLength,
    },
  ].filter((field) => field.element);

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

  const applyI18nValidity = (field) => {
    const el = field.element;
    el.setCustomValidity("");
    const { validity } = el;
    if (validity.valueMissing) {
      el.setCustomValidity(field.required || "");
    } else if (validity.typeMismatch || validity.tooShort || validity.tooLong) {
      el.setCustomValidity(field.invalid || "");
    }
  };

  const syncFieldUi = (field) => {
    applyI18nValidity(field);
    if (field.element.validity.valid) {
      clearField(field);
      return;
    }
    markField(field, field.element.validationMessage);
  };

  const syncAllFieldUi = () => {
    fields.forEach((field) => {
      syncFieldUi(field);
    });
    if (feedback) {
      feedback.hidden = true;
    }
  };

  fields.forEach((field) => {
    field.element.addEventListener("invalid", () => {
      syncAllFieldUi();
    });

    field.element.addEventListener("input", () => {
      field.element.setCustomValidity("");
      if (field.element.getAttribute("aria-invalid") !== "true") {
        return;
      }
      if (!field.element.validity.valid) {
        return;
      }
      clearField(field);
      if (feedback && feedback.dataset.field === field.element.id) {
        feedback.hidden = true;
      }
    });
  });

  form.addEventListener("submit", (event) => {
    fields.forEach((field) => {
      field.element.value = field.element.value.trim();
      applyI18nValidity(field);
    });

    if (form.reportValidity()) {
      fields.forEach((field) => {
        clearField(field);
      });
      return;
    }

    event.preventDefault();
    if (feedback) {
      feedback.hidden = true;
    }
  });
});
