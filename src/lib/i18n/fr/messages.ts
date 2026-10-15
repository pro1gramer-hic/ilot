// messages: texts of this part of the interface (src/lib/i18n/i18n.svelte.ts): the answer to
// a message, in the island's opening and under a thread.

export const messages = {
  reply: {
    to: (name: string) => `Répondre à ${name}`,
    field: (name: string) => `Ta réponse à ${name}`,
    send: "Copier et ouvrir",
    hint: (app: string) => `Entrée copie ta réponse et ouvre ${app}, Échap annule.`,
  },
};
