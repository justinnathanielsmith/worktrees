# GEMINI.md - Instructional Context

## Project Overview
This project, titled **"Giga Chad's Bare Worktree Guide,"** is an elite educational resource and interactive guide for implementing the "Bare Hub Architecture" in Git. It focuses on using Git worktrees to decouple the Git engine from the working directory, allowing for high-performance, parallel development workflows.

### Main Technologies
- **Astro 4.x+**: Modern static site generator for superior performance and componentization.
- **Tailwind CSS**: Utility-first CSS for rapid, consistent styling of the "High-Performance Cyber" UI.
- **Vanilla CSS**: Used for bespoke animations (Aurora, Scanlines, Grain) to avoid the "AI slop" look.
- **TypeScript**: Ensuring type safety across components and logic.
- **Git**: The central subject and core technology of the guide.

### Architecture
The project is a multi-page Astro application employing a component-based architecture. It is designed for maximum visual impact and zero-latency navigation.

- **Routing**: Static routing via `src/pages/` (Home, Architecture, Best Practices, AI Guide, KMP Workflow).
- **Components**: UI logic is decoupled into reusable Astro components (`Hero`, `Protocol`, `CodeBlock`, `FeatureGrid`, etc.).
- **Layouts**: A unified `BaseLayout.astro` manages global styles, fonts (Bebas Neue, Inter, JetBrains Mono), and core aesthetic effects.

## Building and Running
- **Development**: `npm run dev` to start the Astro dev server at `localhost:4321`.
- **Build**: `npm run build` to generate the optimized static site in the `dist/` directory.
- **Preview**: `npm run preview` to test the production build locally.
- **Deployment**: Automated via GitHub Actions to GitHub Pages (`/worktrees` base path).

## Development Conventions
- **Thematic Consistency**: All documentation and UI elements must adhere to the **"High-Performance Cyber"** persona and palette:
    - **Background**: Deep Slate (`#0f172a`)
    - **Primary**: Neon Cyan (`#06b6d4`)
    - **Accent**: Cyber Pink (`#ec4899`)
    - **Success**: Cyber Green (`#10b981`)
    - **Typography**: Industrial Impact (`Bebas Neue`) for headings, clean Sans (`Inter`) for body, and `JetBrains Mono` for code.
- **Visualization**: Use raw neon gradients, glassmorphism, and interactive "Reveal" animations. Avoid standard UI kits.
- **Zero-JS by Default**: Prioritize Astro's static generation. Only use client-side JS for critical interactions (e.g., copy-to-clipboard, intersection observers).
- **Performance**: Maintain a 100/100 Lighthouse score. No heavy dependencies or bloated frameworks.
