const UPLOAD_RESULT_SUCCESS = 0;
const UPLOAD_RESULT_FAILURE = 1;

/**
 * A message box for display a success or a failure message. Hidden by default.
 *
 * Usage in HTML: <info-msg-box id="myInfoBoxMsgId"></info-msg-box>
 */
export default class InfoMsgBox extends HTMLElement {
	/** @type {HTMLDivElement} */
	#messageBox;

	constructor() {
		super();

		const shadow = this.attachShadow({mode: "open"});

		const box = document.createElement("div");
		box.setAttribute("class", "infoMsgBox");

		const text = this.getAttribute("data-text");
		box.textContent = text;

		box.addEventListener("click", (event) => {
			console.log("clicked on ");
			console.log(event);
		});

		const linkEl = document.createElement("link");
		linkEl.setAttribute("rel", "stylesheet");
		linkEl.setAttribute("href", "/index.css");

		this.shadowRoot.append(linkEl, box);

		this.#messageBox = box;
	}

	/* Overrides */

	connectedCallback() {
		this.style.display = "none";
		console.log("Info box attached to ", this);
	}

	/* Methods */

	/**
	 *
	 * @param {string} id
	 * @returns {InfoMsgBox}
	 */
	static getById(id) {
		return /**@type{InfoMsgBox}*/(document.getElementById(id))
	}

	/**
	 * Displays a block with a success colored message
	 *
	 * @param {string} msg - Message to display in the box
	 */
	displaySuccess(msg) {
		this.#displayInfoMsg(msg, UPLOAD_RESULT_SUCCESS);
	}

	/**
	 * Displays a block with a failure colored message
	 *
	 * @param {string} msg - Message to display in the box
	 */
	displayFailure(msg) {
		this.#displayInfoMsg(msg, UPLOAD_RESULT_FAILURE);
	}

	/**
	 * Hides the infoMsg box
	 */
	hide() {
		// let infoMsgEl = document.getElementById("infoMsg");
		this.style.display = "none";
	}

	/**
	 * Displays a block with info operation result.
	 *
	 * @param {string} msg - Message to display in the box
	 * @param {number} result - One of: [UPLOAD_RESULT_SUCCESS, UPLOAD_RESULT_FAILURE]
	 */
	#displayInfoMsg(msg, result) {
		switch (result) {
			case UPLOAD_RESULT_SUCCESS: {
				this.style.display = "block";
				this.#messageBox.className = "infoMsgBox infoMsgSuccess";
				this.#messageBox.innerText = msg;

			}
				break;
			case UPLOAD_RESULT_FAILURE: {
				this.style.display = "block";
				this.#messageBox.className = "infoMsgBox infoMsgFailure";
				this.#messageBox.innerText = msg;

			}
				break;
		}
	}
}

customElements.define("info-msg-box", InfoMsgBox);
