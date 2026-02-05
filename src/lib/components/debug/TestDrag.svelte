<script lang="ts">
  let payload = 'hello';

  function dragstart(e: DragEvent) {
    console.log('dragstart');
    e.dataTransfer?.setData('text/plain', payload);
  }

  function dragover(e: DragEvent) {
    e.preventDefault();
    console.log('dragover firing');
  }

  function drop(e: DragEvent) {
    e.preventDefault();
    console.log('drop', e.dataTransfer?.getData('text/plain'));
  }
</script>

<div draggable="true" on:dragstart={dragstart} style="width:100px;height:100px;background:red;">
  DRAG
</div>

<div
  on:dragover={dragover}
  on:drop={drop}
  style="width:200px;height:200px;background:blue;margin-top:20px;"
>
  DROP
</div>
