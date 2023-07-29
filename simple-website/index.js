"use strict";

import { replaceAt } from "/string_utils.mjs";

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
});
