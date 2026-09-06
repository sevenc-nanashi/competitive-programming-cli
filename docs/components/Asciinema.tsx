/** @jsxImportSource @ox-content/vite-plugin */
import {
  raw,
  renderToString,
  type MarkdownNode,
  type MarkdownTransformer,
} from "@ox-content/vite-plugin";

type Props = { src: string; poster?: string };

export function Asciinema({ src, poster }: Props) {
  return (
    <div role="region" aria-label="Terminal recording">
      <link
        rel="stylesheet"
        href="https://cdn.jsdelivr.net/npm/asciinema-player@3.17.0/dist/bundle/asciinema-player.css"
      />
      <script src="https://cdn.jsdelivr.net/npm/asciinema-player@3.17.0/dist/bundle/asciinema-player.min.js"></script>
      <div data-asciinema-src={src} data-asciinema-poster={poster}></div>
      <script>
        {raw(`(() => {
          const container = document.currentScript.previousElementSibling;
          AsciinemaPlayer.create(container.dataset.asciinemaSrc, container, {
            controls: true,
            poster: container.dataset.asciinemaPoster,
          });
        })();`)}
      </script>
    </div>
  );
}

// SSG renders MDX as HTML; expand the component through Ox Content's AST hook.
export const asciinema: MarkdownTransformer = {
  name: "asciinema",
  transform(ast) {
    function visit(node: MarkdownNode): MarkdownNode {
      if (node.type === "mdxJsxFlowElement" && node.name === "Asciinema") {
        const attributes = node.attributes as { name: string; value: string }[];
        const props = Object.fromEntries(
          attributes.map(({ name, value }) => [name, value]),
        ) as Props;
        return { type: "html", value: renderToString(<Asciinema {...props} />) };
      }
      if (node.children) node.children = node.children.map(visit);
      return node;
    }
    return visit(ast);
  },
};
