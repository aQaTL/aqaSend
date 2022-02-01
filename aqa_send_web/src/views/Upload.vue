<template>
  <div id="upload">
    <h2>Upload file</h2>
    <div class="fileUpload">
      <input type="file" name="files" ref="filePicker" multiple>
      <input type="button" value="Upload" class="bigButton" @click="onUploadFiles">
    </div>
    <div id="orText" class="lineAroundText">Or</div>
    <h2>Upload text</h2>
    <div class="pasteUpload">
      <input type="text" v-model="pasteFilename" placeholder="File name">
      <textarea v-model="pasteText" placeholder="Paste your stuff here"></textarea>
      <input type="button" value="Upload" class="bigButton" @click="onUploadPaste">
    </div>
  </div>
</template>

<script lang="ts">
import {defineComponent, ref} from "vue";
import {API_ENDPOINT, uploadFile, UploadParams, Visibility} from "@/api";

export default defineComponent({
  setup() {
    const filePicker = ref<HTMLInputElement>()
    return {filePicker};
  },

  data() {
    return {
      API_ENDPOINT: API_ENDPOINT,

      files: [],

      pasteText: "",
      pasteFilename: "",
    };
  },

  methods: {
    async onUploadFiles() {
      const files = this.filePicker?.files;
      if (!files) {
        console.error("filePicker is undefined");
        return;
      }

      const uploadParams: UploadParams = {
        visibility: Visibility.public,
        lifetime: "infinite",
        downloadCount: "1",
        password: "none",
      };

      for (let i = 0; i < files.length; i++) {
        await uploadFile(files[i], files[i].name, uploadParams);
      }
    },

    async onUploadPaste() {
      const uploadParams: UploadParams = {
        visibility: Visibility.public,
        lifetime: "infinite",
        downloadCount: "1",
        password: "none",
      };

      let filename = this.pasteFilename;
      if (filename.trim().length == 0) {
        filename = "untitled.txt";
      }

      await uploadFile(this.pasteText, filename, uploadParams);
    }
  }
});
</script>

<style lang="scss" scoped>
@import url("../fonts.scss");

#upload {
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: center;
}

.fileUpload {
  display: grid;
  gap: 10px;
  grid-template-columns: minmax(800px, 1fr);

  input[type=file]::file-selector-button {
    @extend .bigButton;
  }

  input[type=file] {
    font-family: "Nunito Sans", sans-serif;
    color: black;
    font-size: 16pt;
  }
}

.pasteUpload {
  display: grid;
  grid-template-columns: minmax(800px, 1fr);
  gap: 1em;

  input[type=text] {
    color: #ae9ff5;
    font-size: 16pt;
    background-color: #2C3E50;
    border: 1px solid black;
    border-radius: 5px;
    font-family: "Cascadia Code", monospace;
  }

  textarea {
    height: 400px;
    background-color: #2c3e50;
    color: #ae9ff5;
    border: 1px solid black;
    border-radius: 5px;
    font-family: "Cascadia Code", monospace;
  }
}

.bigButton {
  padding: 10px;
  font-family: "Nunito Sans", sans-serif;
  font-weight: bold;
  font-size: 14pt;
  background-color: #ae9ff5;
  border: 1px solid black;
  border-radius: 5px;

  &:hover {
    background-color: darken(#ae9ff5, 5%);
  }
}

#orText {
  font-size: 1.5em;
  font-weight: bold;
}

.lineAroundText {
  display: flex;
  flex-direction: row;
  width: 100%;
}
.lineAroundText:before, .lineAroundText:after{
  content: "";
  flex: 1 1;
  border-bottom: 1px solid #ae9ff5;
  margin-top: auto;
  margin-bottom: auto;
}
.lineAroundText:before {
  margin-right: 10px;
}
.lineAroundText:after {
  margin-left: 10px;
}

</style>