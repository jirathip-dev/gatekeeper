create policy notes_read on public.notes for select using (owner = auth.uid());
