<script lang="ts">
  // 他的日记（Archive 纸面档案调 · §5.6② 首次实拍的主角页）。
  //
  // 空态即契约（00-PHILOSOPHY §6 原则 5）：契约 §5 无日记端点——
  // 后端排查结论：presence stance 的 dreaming_consolidation 是姿态枚举不是日记文本，
  // events 总线无 diary/journal 帧，panel 无日记内省 API。
  // 所以此页是诚实的空态契约页：纸面调完整落地，0 装不造假日记。
  // 当后端真有了日记数据源，这页接真的——空态文案就是它的到来契约。
</script>

<section class="diary-view" aria-label="他的日记">
  <span class="diary-ghost" aria-hidden="true">01</span>

  <div class="diary-sheet">
    <p class="diary-eyebrow">他 的 日 记</p>
    <span class="diary-gold-rule" aria-hidden="true"></span>

    <div class="diary-empty">
      <span class="diary-diamond" aria-hidden="true"></span>
      <!-- 他的声音 = 衬线引语级（§6.1）：日记是被「写」下来的，纸面是诚实的载体 -->
      <p class="diary-voice">
        当他开始写日记时，这里会出现——<br />
        他愿意留下的每一天。
      </p>
    </div>
  </div>

  <p class="diary-truth">
    契约 §5 无日记端点——此页为空态契约，不是预览；他写下的第一页，会真的出现在这。
  </p>
</section>

<style>
  /* Archive 纸面档案调（§5.6②）：纸白底 + 墨黑 + 幽灵编号 + 橙菱形标记。
     调性纪律 2：只发生在页面层——抽屉合上即回到场景调。 */
  .diary-view {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--ap-register-archive-paper);
    color: var(--ap-register-archive-ink);
    overflow: hidden;
  }

  /* 巨型幽灵编号（archive.ghost #c9c8c4）：一册日志的页码感 */
  .diary-ghost {
    position: absolute;
    top: -18px;
    right: 6px;
    font-family: var(--ap-font-mono);
    font-size: clamp(180px, 30vw, 320px);
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.05em;
    color: var(--ap-register-archive-ghost-number);
    pointer-events: none;
    user-select: none;
  }

  .diary-sheet {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 22px;
    padding: 40px 32px;
  }

  /* 引题（§6.1：11px / 字距 0.5em；纸面墨色 55%） */
  .diary-eyebrow {
    margin: 0;
    font-size: 11px;
    letter-spacing: 0.5em;
    color: var(--ap-register-archive-ink-55);
  }

  /* 金色纪律跨调性（§5.6 纪律 3）：强调 = 存在金。
     实拍校准：纸白上 #ffd27a 文字不可读 → 强调落为实体金色短线（金配墨）。 */
  .diary-gold-rule {
    width: 44px;
    height: 3px;
    background: var(--ap-register-archive-accent);
    border-radius: 2px;
  }

  .diary-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 26px;
    margin-top: 12px;
  }

  /* 橙菱形：标记/未读职能（§5.6 纪律 3：不承担强调） */
  .diary-diamond {
    width: 10px;
    height: 10px;
    transform: rotate(45deg);
    background: var(--ap-register-archive-marker-orange);
  }

  /* 他的声音：衬线引语级排版（§6.1 实值 token） */
  .diary-voice {
    margin: 0;
    font-family: var(--ap-font-voice);
    font-size: var(--ap-voice-size);
    line-height: var(--ap-voice-line-height);
    letter-spacing: var(--ap-voice-letter-spacing);
    max-width: 31ch;
    text-align: center;
    color: var(--ap-register-archive-ink);
  }

  /* 0 装标注（§5.4 同范式：等宽小字，用户问「这是真的吗」永远答得起） */
  .diary-truth {
    flex: none;
    margin: 0;
    padding: 14px 24px 18px;
    font-family: var(--ap-font-mono);
    font-size: 10px;
    letter-spacing: 0.28em;
    color: var(--ap-register-archive-ink-40);
    text-align: center;
  }

  @media (max-width: 600px) {
    .diary-voice {
      font-size: var(--ap-voice-size-mobile);
      line-height: var(--ap-voice-line-height-mobile);
    }
  }
</style>
