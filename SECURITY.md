# Security policy

## Supported versions

Only the latest version on the `main` branch receives fixes.

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

Report privately through GitHub instead: go to the repository's **Security** tab and choose **Report a vulnerability**. Include:

- what the problem is and what an attacker could do with it;
- a Jawa program, or other input, that reproduces it;
- your operating system, and how you built `jawac`.

You should get an acknowledgement within a few days. Once a fix ships, the report can be disclosed, with credit to you if you'd like.

## Scope

The most relevant issues are:

- **Unsound generated code:** a Jawa program that reads or writes memory it shouldn't, despite bounds, null and type checks; or a program whose behaviour changes with the optimization level because the compiler relied on undefined behaviour.
- **Runtime memory safety:** buffer overflows or use-after-free in `runtime/jawa_rt.c`, for example through very long strings or arrays.
- **The compiler on hostile input:** a source file that crashes `jawac`, makes it run without end, or makes it use unbounded memory.

Running a Jawa program runs its code with your permissions, the same as any other executable; that is by design. Treat programs from untrusted sources the way you would any other program.
