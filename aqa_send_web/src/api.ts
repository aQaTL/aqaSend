export const API_ENDPOINT =
	process.env.NODE_ENV === "production" ? "notsureyet" : "http://127.0.0.1:8000";

type DOWNLOAD_COUNT_HEADER_VALUE = "1" | "5" | "10" | "100" | "infinite";

enum DownloadCount {
	infinite  = "infinite",
	count_1   = "1",
	count_5   = "5",
	count_10  = "10",
	count_100 = "100",
}

export async function uploadFile(file: File): Promise<boolean> {
	console.debug(`Uploading ${file.name}`);

	let form = document.createElement("form");
	form.enctype = "multipart/form-data";

	let formData = new FormData(form);
	formData.append("file", file, file.name);

	try {
		let response = await fetch(`${API_ENDPOINT}/api/upload`, {
			method: "POST",
			headers: {
				"aqa-download-count": DownloadCount.count_1,
			},
			body: formData,
		});

		console.log("Response: ", response);
		return response.status == 200;
	} catch (ex) {
		console.error(ex);
		return false;
	}
}