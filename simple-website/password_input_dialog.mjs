"use strict";

/**
 *
 * Usage in HTML: <dialog is="password-input-dialog"></dialog>
 *
 * On successful input, emits "passwordInputDone" event to the element passed to the `show` method.
 */
export default class PasswordInput extends HTMLDialogElement {
	#passwordInputDialogTarget;

	constructor() {
		super();

		this.innerHTML = `
			<form>
				<p>
					<label>
						Password:
						<input type="password" placeholder="password" />
					</label>
				</p>
				<div>
					<button type="submit" id="confirmBtn" value="confirm" formmethod="dialog">
						Download
					</button>
					<button type="submit" value="cancel" formmethod="dialog">
						Cancel
					</button>
				</div>
			</form>
		`;

		const inputEl = /** @type {HTMLSelectElement} */ (this.querySelector("input[type=password]"));
		const confirmBtn = /** @type {HTMLButtonElement} */ (this.querySelector("#confirmBtn"));

		inputEl.addEventListener("change", (_event) => {
			this.returnValue = inputEl.value;
		});

		let self = this;
		this.addEventListener("close", (_event) => {
			let password = inputEl.value;

			if ((!self.returnValue) || self.returnValue === "cancel"
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

			this.#passwordInputDialogTarget.dispatchEvent(inputDoneEvent);
		});

		this.addEventListener("cancel", (event) => {
			this.returnValue = "cancel";
		});

		confirmBtn.addEventListener("click", (event) => {
			event.preventDefault();
			this.close("confirm");
		});
	}

	/**
	 *
	 * @param {string} id
	 * @returns {PasswordInput}
	 */
	static getById(id) {
		return /**@type{PasswordInput}*/(document.getElementById(id))
	}

	/**
	 *
	 * @param {HTMLElement} element
	 */
	prompt(element) {
		this.#passwordInputDialogTarget = element;
		this.showModal();
	}
}

customElements.define("password-input", PasswordInput, { extends: "dialog" });