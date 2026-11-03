const key = "tessera-theme";
const media = matchMedia("(prefers-color-scheme: dark)");
let choice = localStorage.getItem(key) || "system";
function apply() {
  document.documentElement.dataset.theme = choice === "system" ? (media.matches ? "dark" : "light") : choice;
  const button = document.querySelector(".theme-toggle");
  if (button) button.title = `Aparência: ${choice === "system" ? "sistema" : choice === "dark" ? "escura" : "clara"}`;
}
document.querySelector(".theme-toggle")?.addEventListener("click", () => {
  choice = choice === "system" ? "light" : choice === "light" ? "dark" : "system";
  localStorage.setItem(key, choice); apply();
});
media.addEventListener("change", apply);
apply();
