// ==UserScript==
// @name         codeforces-clipboard-submission
// @namespace    https://github.com/sevenc-nanashi/competitive-programming-cli
// @version      0.1.0
// @description  Adds a button use clipboard content as submission code in Codeforces problem submission page
// @author       Nanashi.
// @match        http://codeforces.com/contest/*/problem/*
// @match        http://codeforces.com/gym/*/problem/*
// @match        http://codeforces.com/problemset/problem/*
// @match        http://codeforces.com/group/*/contest/*/problem/*
// @match        http://*.contest.codeforces.com/group/*/contest/*/problem/*
// @match        https://codeforces.com/contest/*/problem/*
// @match        https://codeforces.com/gym/*/problem/*
// @match        https://codeforces.com/problemset/problem/*
// @match        https://codeforces.com/group/*/contest/*/problem/*
// @match        https://*.contest.codeforces.com/group/*/contest/*/problem/*
// @grant        none
// @updateURL    https://sevenc7c.com/competitive-programming-cli/codeforces-clipboard-submission.user.js
// ==/UserScript==

void (async () => {
  let submitForm = document.querySelector(".submitForm");
  for (let i = 0; i < 100 && !submitForm; i++) {
    submitForm = document.querySelector(".submitForm");
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  if (!submitForm) {
    console.error("Submit form not found");
    return;
  }
  const submitButton = submitForm.querySelector("input[type=submit]");

  const pasteButton = document.createElement("button");
  pasteButton.style.fontSize = "1.1rem";
  pasteButton.style.width = "17em";
  pasteButton.textContent = "Paste from Clipboard";

  const newRow = document.createElement("tr");
  const newCell1 = document.createElement("td");
  newCell1.className = "field";
  newCell1.textContent = "...or:";
  const newCell2 = document.createElement("td");
  newCell2.appendChild(pasteButton);
  newRow.appendChild(newCell1);
  newRow.appendChild(newCell2);

  submitForm.querySelector("table tbody").insertBefore(newRow, submitButton.closest("tr"));

  pasteButton.addEventListener("click", async (e) => {
    e.preventDefault();
    try {
      const text = await navigator.clipboard.readText();
      const fileInput = submitForm.querySelector("input[name=sourceFile]");
      const blob = new Blob([text], { type: "text/plain" });
      const file = new File([blob], "submission.txt", { type: "text/plain" });
      const dataTransfer = new DataTransfer();
      dataTransfer.items.add(file);
      fileInput.files = dataTransfer.files;
    } catch (err) {
      alert("Failed to read clipboard contents: " + err);
    }
  });
})();
