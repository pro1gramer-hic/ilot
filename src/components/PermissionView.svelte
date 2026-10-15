<!-- The island opened on its own for a permission (560 × auto, 620 for a risky command),
     or for an agent's questions: the full card, until answered. Its height sets the
     island's. -->
<script lang="ts">
  import { i18n } from "../lib/i18n/i18n.svelte";
  import { island } from "../lib/island/controller.svelte";
  import { asksQuestions, permissions } from "../lib/stores/permissions.svelte";
  import PermissionCard from "./PermissionCard.svelte";
  import QuestionCard from "./QuestionCard.svelte";

  const request = $derived(island.permission);
  const answer = $derived(
    request && permissions.answered?.request.id === request.id ? permissions.answered.answer : null,
  );
</script>

<div role="alertdialog" aria-label={request && asksQuestions(request) ? i18n.t.island.question.dialog : i18n.t.island.permission.dialog}>
  {#if request && asksQuestions(request)}
    <QuestionCard {request} {answer} surface="island" waiting={Math.max(0, permissions.queue.length - 1)} />
  {:else if request}
    <PermissionCard {request} {answer} surface="island" waiting={Math.max(0, permissions.queue.length - 1)} />
  {/if}
</div>
