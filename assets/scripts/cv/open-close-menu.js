document.addEventListener("DOMContentLoaded", () => {
  const menuLink = document.querySelector("a#menu");
  if (!!menuLink) {
    const iconMenuLink = menuLink.querySelector("use");
    if (window.location.pathname == "/menu") {
      iconMenuLink.setAttribute(
        "xlink:href",
        "/assets/icons/cv/website/sprites.svg#close-menu"
      );
      menuLink.addEventListener("click", (event) => {
        event.preventDefault();
        window.history.go(-1);
      });
    }
  }
});
