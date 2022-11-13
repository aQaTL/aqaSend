<template>
  <div>
    <NForm id="form">
      <NFormItem label="Username" path="username">
        <NInput type="text" v-model:value="login.username" placeholder="Username" />
      </NFormItem>
      <NFormItem label="password" path="password">
        <NInput type="password" v-model:value="login.password" placeholder="Password" />
      </NFormItem>
      <NFormItem label="Login">
        <NButton type="success">Login</NButton>
      </NFormItem>
    </NForm>
  </div>
</template>

<script lang="ts">
import {defineComponent} from "vue";
import {NButton, NForm, NFormItem, NInput} from "naive-ui";
import {Login} from "@/api";

export default defineComponent({
  name: "Account",

  components: {
    NButton,
    NForm,
    NFormItem,
    NInput,
  },

  data() {
    return {
      login: {
        username: "",
        password: ""
      } as Login
    }
  }
});
</script>

<style scoped>
#form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: center;
  margin-top: 1em;
}
</style>