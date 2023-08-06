"use strict";

/** @type {HTMLElement} */
let passwordInputDialogTarget = null;

export function setup() {
	const dialogEl = /** @type {HTMLDialogElement} */ (document.getElementById("passwordInput"));
	const inputEl = /** @type {HTMLSelectElement} */ (dialogEl.querySelector("input[type=password]"));
	const confirmBtn = /** @type {HTMLButtonElement} */ (dialogEl.querySelector("#confirmBtn"));

	inputEl.addEventListener("change", (_event) => {
		dialogEl.returnValue = inputEl.value;
	});

	dialogEl.addEventListener("close", (_event) => {
		let password = inputEl.value;

		if ((!dialogEl.returnValue) || dialogEl.returnValue === "cancel"
			|| password.length === 0)
		{
			console.log("dialog cancelled");
			return;
		}

		let inputDoneEvent = new CustomEvent("passwordInputDone", {
			detail: {
				password: password,
			},
			bubbles: true,
			cancelable: true,
			composed: false,
		});

		passwordInputDialogTarget.dispatchEvent(inputDoneEvent);
	});

	dialogEl.addEventListener("cancel", (event) => {
		dialogEl.returnValue = "cancel";
	});

	confirmBtn.addEventListener("click", (event) => {
		event.preventDefault();
		dialogEl.close("confirm");
	});
}

/**
 *
 * @param {HTMLElement} element
 */
export function show(element) {
	passwordInputDialogTarget = element;

	const dialogEl = /** @type {HTMLDialogElement} */ (document.getElementById("passwordInput"));
	dialogEl.showModal();
}
