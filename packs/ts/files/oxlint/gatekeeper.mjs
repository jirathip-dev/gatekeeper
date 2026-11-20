const noEnum = {
  meta: { type: "problem" },
  create(context) {
    return {
      TSEnumDeclaration(node) {
        context.report({
          node,
          message: "enum is banned: use a union of string literals or an `as const` object",
        });
      },
    };
  },
};

export default { meta: { name: "gatekeeper" }, rules: { "no-enum": noEnum } };
