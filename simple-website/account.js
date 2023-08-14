import { API_SERVER } from "./api_endpoint.mjs";
import * as Api from "./api.mjs";
import InfoMsgBox from "./info_msg_box/info_msg_box.mjs";

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
	loadUser();

	document.getElementById("generateCodeBtn").addEventListener("click", generateCode);
});

async function generateCode(_event) {
	let messageBox = InfoMsgBox.getById("messageBox");
	try {
		let response = await fetch(`${API_SERVER}/api/registration_code`, {
			credentials: "include"
		});
		let responseText = await response.text();
		if (response.status !== 201) {
			messageBox.displayFailure(responseText);
			return;
		}
		showOutput(responseText);
	} catch(ex) {
		console.error(ex);
	}
}

/**
 *
 * @param {string} generatedCode
 */
function showOutput(generatedCode) {
	document.getElementById("generatedCodeOutput").style.display = "block";
	let link = `${window.location.origin}/registration.html?invite=${encodeURIComponent(generatedCode)}`;

	let generatedCodeLinkEl = /** @type {HTMLLinkElement} */ (document.getElementById("generatedCodeLink"));
	generatedCodeLinkEl.href = link;
	generatedCodeLinkEl.innerText = link;
}