create table public.notes (
  id bigint generated always as identity primary key,
  owner uuid not null default auth.uid(),
  body text not null
);
alter table public.notes enable row level security;
create policy notes_owner on public.notes for all using (owner = (select auth.uid()));
