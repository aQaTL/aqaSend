"use strict";

import { API_SERVER } from "./api_endpoint.mjs";
import * as Api from "./api.mjs";
import * as Types from "./models.mjs";
import InfoMsgBox from "./info_msg_box/info_msg_box.mjs";

function main() {
	let loginFormEl = document.getElementById("loginForm");
	loginFormEl.addEventListener("submit", submitLoginForm);
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

	const resultBox = InfoMsgBox.getById("loginResult");

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		if (request.status !== 201) {
			/** @type {Types.ErrorJsonBody} */
			const response = request.response;
			resultBox.displayFailure(response.message);
		} else {
			resultBox.displaySuccess("Logged in successfully");
		}
	});

	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/login`);
	request.send(formData);
}
