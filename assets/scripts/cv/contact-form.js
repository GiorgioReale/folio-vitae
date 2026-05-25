document.addEventListener("DOMContentLoaded", () => {
  const form = document.querySelector(".contact-form [data-contact-form]");
  if (!form) {
    return;
  }
  const fullNameField = form.querySelector("#full-name");
  const companyField = form.querySelector("#company");
  const emailField = form.querySelector("#email");
  const messageField = form.querySelector("#message");
  const feedback = document.querySelector("[data-contact-feedback]");
  const FEEDBACK_SUCCESS = "contact-form__feedback--success";
  const FEEDBACK_ERROR = "contact-form__feedback--error";
  const LIMITS = {
    fullName: { min: 3, max: 120 },
    company: { min: 2, max: 120 },
    email: { min: 5, max: 254 },
    message: { min: 10, max: 2000 },
  };
  const EMAIL_REGEX = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;
  const fields = [fullNameField, companyField, emailField, messageField].filter(
    Boolean
  );
  const setFeedback = (message, statusClass) => {
    if (!feedback) {
      return;
    }
    const messageTarget = feedback.querySelector("p");
    if (!message) {
      feedback.hidden = true;
      feedback.classList.remove(FEEDBACK_SUCCESS, FEEDBACK_ERROR);
      if (messageTarget) {
        messageTarget.textContent = "";
      }
      return;
    }
    feedback.hidden = false;
    feedback.classList.remove(FEEDBACK_SUCCESS, FEEDBACK_ERROR);
    if (statusClass) {
      feedback.classList.add(statusClass);
    }
    if (messageTarget) {
      messageTarget.textContent = message;
    }
  };
  const markInvalid = (field) => {
    if (!field) {
      return;
    }
    field.classList.add("is-invalid");
    field.setAttribute("aria-invalid", "true");
  };
  const clearInvalid = (field) => {
    if (!field) {
      return;
    }
    field.classList.remove("is-invalid");
    field.removeAttribute("aria-invalid");
  };
  const validate = () => {
    const values = {
      fullName: fullNameField?.value.trim() ?? "",
      company: companyField?.value.trim() ?? "",
      email: emailField?.value.trim() ?? "",
      message: messageField?.value.trim() ?? "",
    };
    let errorMessage = "";
    fields.forEach((field) => clearInvalid(field));
    if (
      !values.fullName ||
      !values.company ||
      !values.email ||
      !values.message
    ) {
      errorMessage = "Compila tutti i campi prima di inviare.";
    } else if (
      values.fullName.length < LIMITS.fullName.min ||
      values.fullName.length > LIMITS.fullName.max
    ) {
      errorMessage = `Il nome e cognome deve contenere tra ${LIMITS.fullName.min} e ${LIMITS.fullName.max} caratteri.`;
    } else if (
      values.company.length < LIMITS.company.min ||
      values.company.length > LIMITS.company.max
    ) {
      errorMessage = `Il nome dell'azienda deve contenere tra ${LIMITS.company.min} e ${LIMITS.company.max} caratteri.`;
    } else if (
      values.email.length < LIMITS.email.min ||
      values.email.length > LIMITS.email.max ||
      !EMAIL_REGEX.test(values.email)
    ) {
      errorMessage = "Inserisci un indirizzo email valido.";
    } else if (
      values.message.length < LIMITS.message.min ||
      values.message.length > LIMITS.message.max
    ) {
      errorMessage = `Il messaggio deve contenere tra ${LIMITS.message.min} e ${LIMITS.message.max} caratteri.`;
    }
    if (errorMessage) {
      if (!values.fullName) {
        markInvalid(fullNameField);
      }
      if (!values.company) {
        markInvalid(companyField);
      }
      if (!values.email) {
        markInvalid(emailField);
      }
      if (!values.message) {
        markInvalid(messageField);
      }
      if (errorMessage.includes("nome e cognome")) {
        markInvalid(fullNameField);
      } else if (errorMessage.includes("azienda")) {
        markInvalid(companyField);
      } else if (errorMessage.includes("email")) {
        markInvalid(emailField);
      } else if (errorMessage.includes("messaggio")) {
        markInvalid(messageField);
      }
    }
    return { values, errorMessage };
  };
  form.addEventListener("submit", (event) => {
    const { values, errorMessage } = validate();
    if (errorMessage) {
      event.preventDefault();
      setFeedback(errorMessage, FEEDBACK_ERROR);
      const firstInvalid = fields.find((field) =>
        field?.classList.contains("is-invalid")
      );
      firstInvalid?.focus();
      return;
    }
    setFeedback("", null);
    if (fullNameField) {
      fullNameField.value = values.fullName;
    }
    if (companyField) {
      companyField.value = values.company;
    }
    if (emailField) {
      emailField.value = values.email;
    }
    if (messageField) {
      messageField.value = values.message;
    }
  });
  fields.forEach((field) => {
    field?.addEventListener("input", () => {
      clearInvalid(field);
      setFeedback("", null);
    });
  });
});
