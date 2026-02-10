<script>
  export let groups = [
    {
      name: 'Fruit basket 1',
      items: ['Orange', 'Pineapple'],
    },
    {
      name: 'Fruit basket 2',
      items: ['Banana', 'Apple'],
    },
  ];

  export function dragstart(ev, group, item) {
    ev.dataTransfer.setData('group', group);
    ev.dataTransfer.setData('item', item);
  }

  export function dragover(ev) {
    ev.preventDefault();
    ev.dataTransfer.dropEffect = 'move';
  }

  export function drop(ev, new_g) {
    ev.preventDefault();
    var i = ev.dataTransfer.getData('item');
    var old_g = ev.dataTransfer.getData('group');
    const item = groups[old_g].items.splice(i, 1)[0];
    groups[new_g].items.push(item);
    groups = groups;
  }
</script>

<p>Please drag a fruit from one basket to the other!</p>

{#each groups as group, g}
  <b>{group.name}</b>
  <ul on:drop={event => drop(event, g)} on:dragover={dragover}>
    {#each group.items as item, i}
      <li draggable={true} on:dragstart={event => dragstart(event, g, i)}>{item}</li>
    {/each}
  </ul>
{/each}
