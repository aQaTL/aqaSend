<template>
  <div id="uploadParams">
    <h2 class="lineAroundText">Parameters</h2>
    <div class="param">
      <NSwitch
          size="large"
          :checked-value="Visibility.private"
          :unchecked-value="Visibility.public"
          :value="uploadParams.visibility"
          @update:value="(v) => uploadParams.visibility = v"
      >
        <template #checked>
          Private
        </template>
        <template #unchecked>
          Public
        </template>
      </NSwitch>
    </div>
    <div class="param downloadCount">
      <span>Download count</span>
      <NSlider v-model:value="uploadParams.downloadCount" :marks="downloadCounts" step="mark" ></NSlider>
      <span>Infinity download count toggle</span>
      <NSwitch
          size="large"
        >
        <template #checked>
          Infinite
        </template>
        <template #unchecked>
          {{ uploadParams.downloadCount }}
        </template>
      </NSwitch>
    </div>
    <div class="param">
      <span>Lifetime</span>

    </div>
    <div class="param">
      <span>Password</span>
      <input type="password" placeholder="Leave empty for unprotected file"
             v-model="uploadParams.password">
      <NInput
          type="password"
          placeholder="Leave empty for unprotected file"
          show-password-on="mousedown"
          clearable
        ></NInput>

    </div>
  </div>
  <div id="upload">
    <h2 class="lineAroundText">Upload file</h2>
    <div class="fileUpload">
      <input type="file" name="files" ref="filePicker" multiple>
      <input type="button" value="Upload" class="bigButton" @click="onUploadFiles">
    </div>
    <h2 class="lineAroundText">Upload text</h2>
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
import {NSwitch, NSlider, NInput} from "naive-ui";

export default defineComponent({
  components: {
    NSwitch,
    NSlider,
    NInput,
  },

  setup() {
    const filePicker = ref<HTMLInputElement>()
    return {filePicker};
  },

  data() {
    return {
      API_ENDPOINT: API_ENDPOINT,
      Visibility: Visibility,

      files: [],

      pasteText: "",
      pasteFilename: "",

      uploadParams: {
        visibility: Visibility.public,
        downloadCount: "1",
        password: "",
      } as UploadParams,

      downloadCounts: {
        1: "1",
        5: "5",
        10: "10",
        100: "100",
      }
    };
  },

  methods: {
    async onUploadFiles() {
      const files = this.filePicker?.files;
      if (!files) {
        console.error("filePicker is undefined");
        return;
      }

      for (let i = 0; i < files.length; i++) {
        await uploadFile(files[i], files[i].name, this.uploadParams);
      }
    },

    async onUploadPaste() {
      let filename = this.pasteFilename;
      if (filename.trim().length === 0) {
        filename = "untitled.txt";
      }

      let blob = new Blob([this.pasteText], {type: "text/plain"});
      await uploadFile(blob, filename, this.uploadParams);
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

  input[type=file] {
    font-family: "Nunito Sans", sans-serif;
    color: black;
    font-size: 16pt;
  }

  input[type=file]::file-selector-button {
    @extend .bigButton;
  }

  input[type=file]::-webkit-file-upload-button {
    @extend .bigButton;
  }

}

.pasteUpload {
  display: grid;
  grid-template-columns: minmax(800px, 1fr);
  gap: 1em;

  input[type=text] {
    color: #AE9FF5;
    font-size: 16pt;
    background-color: #2C3E50;
    border: 1px solid black;
    border-radius: 5px;
    font-family: "Cascadia Code", monospace;
  }

  textarea {
    height: 400px;
    background-color: #2C3E50;
    color: #AE9FF5;
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
  background-color: #AE9FF5;
  border: 1px solid black;
  border-radius: 5px;

  &:hover {
    background-color: darken(#AE9FF5, 5%);
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

.lineAroundText:before, .lineAroundText:after {
  content: "";
  flex: 1 1;
  border-bottom: 1px solid #AE9FF5;
  margin-top: auto;
  margin-bottom: auto;
}

.lineAroundText:before {
  margin-right: 10px;
}

.lineAroundText:after {
  margin-left: 10px;
}

.downloadCount {
  display: flex;
  flex-direction: column;
  align-items: center;
  height: 200px;
}

.downloadCount > .n-slider {
  max-width: 500px;
}

</style>