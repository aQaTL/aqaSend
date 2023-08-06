"use strict";

import * as Types from "./models.mjs"
import {API_SERVER} from "./api.mjs";

function hello() {
	let greetingEl = document.getElementById("greeting");
	greetingEl.innerHTML = "All files";
}

window.addEventListener("DOMContentLoaded", function (_event) {
	hello();
	loadFiles();
	setupPasswordInputDialog();
});

function loadFiles() {
	console.log("Loading files");

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		/** @type {[Types.FileModel]} */
		const response = request.response;
		console.log("success: " + JSON.stringify(response));

		displayFiles(response);
	});

	request.responseType = "json";
	request.open("GET", `${API_SERVER}/api/list.json`);
	request.send();
}

/**
 * Renders file list
 * @param {Types.FileModel[]} files - list of files
 */
function displayFiles(files) {
	let fileEntriesEl = document.getElementById("fileEntries");

	for (let i = 0; i < files.length; i++) {
		const file = files[i];

		let fileEntryEl = document.createElement("div");
		fileEntryEl.className = "fileEntry";

		let fileLinkEl = document.createElement("a");
		if (!file.has_password) {
			fileLinkEl.href = `${API_SERVER}/api/download/${file.uuid}`;
		} else {
			fileLinkEl.href = "javascript:void(0);";

			fileLinkEl.addEventListener("click", async (_) => {
				showPasswordInputDialog(fileLinkEl);
			});

			fileLinkEl.addEventListener("passwordInputDone", (/** @type {CustomEvent} */ event) => {
				let password = event.detail.password;
				fileLinkEl.href =
					`${API_SERVER}/api/download/${file.uuid}?password=${encodeURIComponent(password)}`;
				fileLinkEl.click();
			});
		}

		let filenameEl = document.createElement("div");
		filenameEl.className = "fileEntryFilename";
		filenameEl.appendChild(document.createTextNode(file.filename));
		fileLinkEl.appendChild(filenameEl);


		if (file.visibility === "Private") {
			fileEntryEl.className += " fileEntryPrivate";
		}

		let statsEl = document.createElement("div");
		{
			let div = document.createElement("div");

			let lifetime;
			if (file.lifetime === null) {
				lifetime = "Infinite";
			} else {
				lifetime = formatDuration(file.lifetime.secs * 1000);
			}

			div.appendChild(document.createTextNode("lifetime: " + lifetime));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			const uploadDate = new Date(file.upload_date.secs_since_epoch * 1000);
			div.appendChild(document.createTextNode(
				`upload date: ${uploadDate.toLocaleDateString()} ${uploadDate.toLocaleTimeString()}`
			));
			statsEl.appendChild(div);
		}
		{
			let div = document.createElement("div");
			div.appendChild(document.createTextNode(
				"download count: " + file.download_count
			));
			statsEl.appendChild(div);
		}
		fileLinkEl.appendChild(statsEl);

		fileEntryEl.appendChild(fileLinkEl);

		fileEntriesEl.appendChild(fileEntryEl);
	}

}

function formatDuration(ms) {
	const time = {
		day: Math.floor(ms / 86400000),
		hour: Math.floor(ms / 3600000) % 24,
		minute: Math.floor(ms / 60000) % 60,
		second: Math.floor(ms / 1000) % 60,
		millisecond: Math.floor(ms) % 1000
	};
	return Object.entries(time)
		.filter(val => val[1] !== 0)
		.map(([key, val]) => `${val} ${key}${val !== 1 ? "s" : ""}`)
		.join(", ");
}

/** @type {HTMLElement} */
let passwordInputDialogTarget = null;

function setupPasswordInputDialog() {
	const dialogEl = /** @type {HTMLDialogElement} */ (document.getElementById("passwordInput"));
	const inputEl = /** @type {HTMLSelectElement} */ (dialogEl.querySelector("input[type=password]"));
	const confirmBtn = /** @type {HTMLButtonElement} */ (dialogEl.querySelector("#confirmBtn"));

	inputEl.addEventListener("change", (_event) => {
		dialogEl.returnValue = inputEl.value;
	});

	dialogEl.addEventListener("close", (_event) => {
		let password = inputEl.value;

		if ((!dialogEl.returnValue) || dialogEl.returnValue === "cancel"
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

		passwordInputDialogTarget.dispatchEvent(inputDoneEvent);
	});

	dialogEl.addEventListener("cancel", (event) => {
		dialogEl.returnValue = "cancel";
	});

	confirmBtn.addEventListener("click", (event) => {
		event.preventDefault();
		dialogEl.close("confirm");
	});
}

/**
 *
 * @param {HTMLElement} element
 */
function showPasswordInputDialog(element) {
	passwordInputDialogTarget = element;

	const dialogEl = /** @type {HTMLDialogElement} */ (document.getElementById("passwordInput"));
	dialogEl.showModal();
}