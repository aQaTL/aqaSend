"use strict";

import { replaceAt } from "./string_utils.mjs";
import * as Api from "./api.mjs";

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

/** 
 * Says hello
 * @param {string} name - Name of the user to greet
 */
function hello(name) {
	name = replaceAt(name, 0, name[0].toUpperCase());
	let greetingEl = document.getElementById("greeting");
	greetingEl.innerHTML = `Hi, ${name}! Welcome to aQaSend!`;
}

window.addEventListener("DOMContentLoaded", function(_event) {
	hello("bob");
	loadUser();
});
