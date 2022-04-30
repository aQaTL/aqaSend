<template>
  <div class="home">
    <!--    <img alt="Vue logo" src="../assets/logo.png">-->
    <!--    <HelloWorld msg="Welcome to Your Vue.js + TypeScript App"/>-->
    <div>
      <div v-for="fileEntry in this.fileEntries">
        <div><a :href="`${this.API_ENDPOINT}/api/download/${fileEntry.id}`">{{
            fileEntry.filename
          }}</a> {{ fileEntry.download_count }}
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { API_ENDPOINT } from "@/api";

export default defineComponent({
  components: {},

  data() {
    return {
      API_ENDPOINT: API_ENDPOINT,
      COUNTER: 9,
      fileEntries: [],
    }
  },

  mounted() {
    console.log(`NODE_ENV: ${process.env.NODE_ENV}`);
    this.loadList();
  },

  methods: {
    async loadList() {
      let response = await fetch(
          `${API_ENDPOINT}/api/list.json`,
          {
            method: "GET",
            headers: {}
          }
      );
      this.fileEntries = await response.json();
      console.debug(`Loaded ${this.fileEntries.length} files`);
    },

  }
})
</script>

