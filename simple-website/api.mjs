import * as Types from "./models.mjs"
import {API_SERVER} from "./api_endpoint.mjs";

/**
 * Loads list of uploaded files
 *
 * @param {boolean} only_self - when true, only files uploaded by logged-in user will be listed
 * @returns {Promise<Types.FileModel[]>}
 */
export async function loadFiles(only_self = false) {
	console.log("Loading files");

	try {
		let query = only_self ? "uploader=me" : "";
		let response = await fetch(`${API_SERVER}/api/list.json?${query}`, {
			credentials: "include"
		})

		let files =/** @type {Types.FileModel[]} */ (await response.json());
		return files;
	} catch (ex) {
		console.error(ex);
		return [];
	}
}

/**
 *
 * @returns {Promise<string|null>}
 */
export async function loadUser() {
	console.log("Loading user");

	try {
		/** @type {Response} */
		let response = await fetch(`${API_SERVER}/api/whoami`, {
			credentials: "include"
		});
		if (response.status === 200) {
			return await response.text();
		} else {
			return null;
		}
	} catch (ex) {
		console.error(ex);
		return null;
	}
}
