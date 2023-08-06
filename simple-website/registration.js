"use strict";

import { API_SERVER } from "./api_endpoint.mjs";
import * as Api from "./api.mjs";
import * as Types from "./models.mjs";

function main() {
	let loginFormEl = document.getElementById("loginForm");
	loginFormEl.addEventListener("submit", submitRegistrationForm);
}

async function loadUser() {
	let username = /** @type {?string} */ (await Api.loadUser());
	if (username) {
		console.log(`current user: ${username}`);
		let userInfoEl = document.getElementById("userInfo");
		userInfoEl.innerText = username;
		userInfoEl.style.display = "block";
	} else {
		console.log(`No user logged in`);
	}
}

window.addEventListener("DOMContentLoaded", function(_event) {
	let greetingEl = document.getElementById("greeting");
	greetingEl.innerHTML = `Register`;
	main();
	loadUser();
});

/**
 * Tries to request a login
 *
 * @param {SubmitEvent} event - Event fired when clicked on submit
*/
function submitRegistrationForm(event) {
	event.preventDefault();

	console.log("Clicked to submit!");

	const loginFormEl = /** @type {HTMLFormElement} */ (document.getElementById("loginForm"));
	const formData = new FormData(loginFormEl);

	const queryParams = new URLSearchParams(window.location.search);
	const registrationCode = queryParams.get("invite");
	formData.append("registration_code", registrationCode);

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		/** @type {Types.ErrorJsonBody} */
		const response = request.response;
		console.log("success: " + JSON.stringify(response));
	});

	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/create_account`);
	request.send(formData);
}