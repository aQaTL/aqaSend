/**
 * @typedef ErrorJsonBody
 * @type {object}
 * @property {number} status - HTTP status code
 * @property {string} message - error message
*/

/**
 * @typedef UploadedFile
 * @type {object}
 * @property {string} uuid - ''
 * @property {string} filename - ''
*/

/**
 * @typedef FileModel
 * @type {object}
 * @property {string} uuid - file uuid

 * @property {string} filename - 
 * @property {string} content_type -
 * @property {string} uploader_uuid -
 * 
 * @property {DownloadCount} download_count_type - 
 * @property {number} download_count - 
 *
 * @property {Visibility} visibility - 
 * @property {string} password - 
 *
 * @property {Lifetime} lifetime - 
 * @property {Date} upload_date - 
*/

/**
* @typedef Date
* @property {number} secs_since_epoch -
* @property {number} nanos_since_epoch - 
*/


export const Types = {};
