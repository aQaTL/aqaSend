"use strict";

import { API_SERVER } from "./api_endpoint.mjs";
import * as Api from "./api.mjs";
import * as Types from "./models.mjs";
import InfoMsgBox from "./info_msg_box/info_msg_box.mjs";

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

async function checkRegistrationCode() {
	const queryParams = new URLSearchParams(window.location.search);
	const registrationCode = /**@type {string}*/ queryParams.get("invite");
	
	const registrationResult = InfoMsgBox.getById("registrationResult");

	try {
		const response = await fetch(`${API_SERVER}/api/check_registration_code/${registrationCode}`);
		
		if (response.status !== 200) {
			const responseObj = /**@type {Types.ErrorJsonBody}*/(await response.json());
			registrationResult.displayFailure(responseObj.message);
		} else {
			const responseObj = /**@type {Types.CheckRegistrationCodeResponse}*/(await response.json());
			registrationResult.displaySuccess(`Account type: ${responseObj.account_kind}`);
		}
	} catch (ex) {
		console.error(ex);
	}
}

window.addEventListener("DOMContentLoaded", function(_event) {
	main();
	loadUser();
	checkRegistrationCode();
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

	const registrationResult = InfoMsgBox.getById("registrationResult");

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		if (request.status !== 201) {
			/** @type {Types.ErrorJsonBody} */
			const response = request.response;

			registrationResult.displayFailure(response.message);
		} else {
			/** @type {Types.CreateAccountResponse} */
			const response = request.response;
			console.log("success: " + JSON.stringify(response));
			
			registrationResult.displaySuccess("Account created");
		}
	});

	request.responseType = "json";
	request.open("POST", `${API_SERVER}/api/create_account`);
	request.send(formData);
}
