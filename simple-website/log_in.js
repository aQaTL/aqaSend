"use strict";

import { API_SERVER } from "./api_endpoint.mjs";
import * as Api from "./api.mjs";

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

async function loadUser() {
	let username = /** @type {?string} */ (await Api.loadUser());
	if (username) {
		console.log(`current user: ${username}`);
		document.getElementById("userInfo").innerText = username;
	} else {
		console.log(`No user logged in`);
	}
}

window.addEventListener("DOMContentLoaded", function(_event) {
	let greetingEl = document.getElementById("greeting");
	greetingEl.innerHTML = `Log In`;
	main();
	loadUser();
});

/**
 * Tries to request a login
 *
 * @param {SubmitEvent} event - Event fired when clicked on submit
*/
function submitLoginForm(event) {
	event.preventDefault();

	console.log("Clicked to submit!");

	const loginFormEl = /** @type {HTMLFormElement} */ (document.getElementById("loginForm"));
	const formData = new FormData(loginFormEl);

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		/** @type {ErrorJsonBody} */
		const response = request.response;
		console.log("success: " + JSON.stringify(response));
	});

	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/login`);
	request.send(formData);

}
