begin;
create extension if not exists pgtap with schema extensions;
select plan(1);
select has_column('public', 'notes', 'title', 'public.notes has a title');
select * from finish();
rollback;
