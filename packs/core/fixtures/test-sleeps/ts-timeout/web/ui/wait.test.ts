test("waits", async () => {
  await new Promise((done) => setTimeout(done, 10));
});
