"use strict";

import { API_SERVER } from "./api.mjs";

/**
 * @typedef ErrorJsonBody
 * @type {object}
 * @property {number} status - HTTP status code
 * @property {string} message - error message
*/

function main() {
	let loginFormEl = document.getElementById("loginForm");
	loginFormEl.addEventListener("submit", submitLoginForm);
}

window.addEventListener("DOMContentLoaded", function(_event) {
	let greetingEl = document.getElementById("greeting");
	greetingEl.innerHTML = `Log In`;
	main();
});

/**
 * Tries to request a login
 *
 * @param {SubmitEvent} event - Event fired when clicked on submit
*/
function submitLoginForm(event) {
	event.preventDefault();

	console.log("Clicked to submit!");

	const loginFormEl = event.target;
	const formData = new FormData(loginFormEl);

	const request = new XMLHttpRequest();
	request.addEventListener("load", (event) => {
		/** @type {ErrorJsonBody} */
		const response = event.target.response;
		console.log("success: " + JSON.stringify(response));
	});

	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/login`);
	request.send(formData);

}
