<template>
  <div class="upload">
<!--    <form method="post" enctype="multipart/form-data">-->
      <input type="file" name="files" ref="filePicker" multiple>
      <input type="button" value="Upload" @click="upload">
<!--    </form>-->
  </div>
</template>

<script lang="ts">
import {defineComponent, ref} from "vue";
import {API_ENDPOINT, uploadFile} from "@/api";

export default defineComponent({
  setup() {
    const filePicker = ref<HTMLInputElement>()
    return {filePicker};
  },

  data() {
    return {
      API_ENDPOINT: API_ENDPOINT,
      files: [],
    };
  },

  methods: {
    async upload() {
      const files = this.filePicker?.files;
      if (!files) {
        console.error("filePicker is undefined");
        return;
      }

      for (let i = 0; i < files.length; i++) {
        await uploadFile(files[i]);
      }
    }
  }
});
</script>

<style lang="scss" scoped>

</style>