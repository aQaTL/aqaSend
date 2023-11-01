"use strict";

import * as Types from "./models.mjs"
import {API_SERVER} from "./api_endpoint.mjs";
import PasswordInput from "./components/password_input_dialog.mjs";
import InfoMsgBox from "./components/info_msg_box.mjs";
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
	loadMyFiles();
});

async function loadMyFiles() {
	let files = await Api.loadFiles(true);
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
				PasswordInput.getById("passwordInput").prompt(fileLinkEl);
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

		let deleteButtonEl = document.createElement("button");
		deleteButtonEl.type = "button";
		deleteButtonEl.addEventListener("click", (/**@type{Event}*/ev) => {
				ev.preventDefault();
				deleteFileEntry(ev, file);
		});
		deleteButtonEl.appendChild(document.createTextNode("🗑️"))
		deleteButtonEl.className = "deleteButton";
		fileLinkEl.appendChild(deleteButtonEl);

		if (file.visibility === "Private") {
			fileEntryEl.className += " fileEntryPrivate";
		}

		let statsEl = document.createElement("div");
		statsEl.className = "fileEntryStats";
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

/**
 *
 * @param {Event} event
 * @param {Types.FileModel} file
 * @returns {Promise<void>}
 */
async function deleteFileEntry(event, file) {
	const infoBox = InfoMsgBox.getById("infoBox");
	infoBox.displayFailure(`Deleting ${file.filename}`)
	infoBox.scrollIntoView({behavior: "smooth", block: "end"});

	const request = new XMLHttpRequest();
	request.addEventListener("load", (_event) => {
		// We're expecting 204 no content
		if (request.status === 204) {
			infoBox.displaySuccess("Deleted successfully");
		} else {
			/** @type {Types.ErrorJsonBody} */
			const response = request.response;

			console.error("error: " + response && JSON.stringify(response) || "");
			infoBox.displayFailure(response.message);
		}
	});

	request.addEventListener("error", (/**@type{ErrorEvent}*/_event) => {
		/** @type {Types.ErrorJsonBody} */
		const response = request.response;

		console.error("error: " + (response && JSON.stringify(response) || ""));
		infoBox.displayFailure(response?.message ?? _event.message ?? "Failed to delete");
	});

	request.responseType = "json";
	request.open("DELETE", `${API_SERVER}/api/delete/${file.uuid}`);
	request.send();
}
