"use strict";

import * as Types from "./models.mjs"
import {API_SERVER} from "./api_endpoint.mjs";
import * as PasswordInputDialog from "./password_input_dialog.mjs"
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

window.addEventListener("DOMContentLoaded", function (_event) {
	loadUser();
	loadFiles();
	PasswordInputDialog.setup();
});

async function loadFiles() {
	let files = await Api.loadFiles();
	displayFiles(files);
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
				PasswordInputDialog.show(fileLinkEl);
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
