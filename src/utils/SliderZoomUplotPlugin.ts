import type { Ref } from 'vue';
import uPlot from 'uplot';

export function sliderZoomPlugin(config: {
    container: Ref<HTMLElement | null> | HTMLElement;
    init: [number, number];
    height?: number;
}): uPlot.Plugin {
    const containerEl = config.container instanceof HTMLElement ? config.container : config.container.value;

    if (!containerEl) {
        return {} as uPlot.Plugin;
    }

    let uZoomed: uPlot | null = null;
    let uRanger: uPlot | null = null;
    let preserveRelativeZoom = false;
    let relativeZoomState: { leftRatio: number; rightRatio: number } | null = null;

    return {
        opts: (u, opts) => {
            if (u === uZoomed) {
                uPlot.assign(opts, {
                    scales: { x: { auto: false } }
                });
            }
        },

        hooks: {
            setScale: [
                function (this: uPlot, self: uPlot, key: string) {
                    if (self !== uZoomed || key !== 'x' || !uRanger) return;

                    // Skip updating the slider if we're preserving relative zoom during data update
                    if (preserveRelativeZoom) return;

                    const { min, max } = self.scales.x as { min: number; max: number };
                    const toPx = (v: number) => Math.round(uRanger!.valToPos(v, 'x'));
                    const left = Math.max(0, Math.min(toPx(min), uRanger.bbox.width / devicePixelRatio));
                    const right = Math.max(0, Math.min(toPx(max), uRanger.bbox.width / devicePixelRatio));
                    uRanger.setSelect(
                        {
                            left,
                            top: 0,
                            width: right - left,
                            height: uRanger.bbox.height / devicePixelRatio
                        },
                        false
                    );
                }
            ],

            setData: [
                function (this: uPlot, self: uPlot) {
                    if (self !== uZoomed || !uRanger) return;

                    // Store current relative zoom state before updating data
                    const currentSelect = uRanger.select;
                    const rangerWidth = uRanger.bbox.width / devicePixelRatio;

                    if (currentSelect && rangerWidth > 0) {
                        const leftRatio = currentSelect.left / rangerWidth;
                        const rightRatio = (currentSelect.left + currentSelect.width) / rangerWidth;
                        relativeZoomState = { leftRatio, rightRatio };
                    }

                    // Set flag to preserve zoom during the data update
                    preserveRelativeZoom = true;
                    uRanger.setData(self.data, true);

                    // Use a setTimeout to ensure this happens after any other potential setScale calls
                    setTimeout(() => {
                        // Restore relative zoom position after data update
                        if (relativeZoomState && uRanger && uRanger.data[0] && uRanger.data[0].length > 0) {
                            const newRangerWidth = uRanger.bbox.width / devicePixelRatio;
                            const newLeft = relativeZoomState.leftRatio * newRangerWidth;
                            const newRight = relativeZoomState.rightRatio * newRangerWidth;
                            const newWidth = newRight - newLeft;

                            // Ensure the selection stays within bounds
                            const clampedLeft = Math.max(0, Math.min(newLeft, newRangerWidth - 10));
                            const clampedWidth = Math.max(10, Math.min(newWidth, newRangerWidth - clampedLeft));

                            uRanger.setSelect(
                                {
                                    left: clampedLeft,
                                    top: 0,
                                    width: clampedWidth,
                                    height: uRanger.bbox.height / devicePixelRatio
                                },
                                true
                            );
                        }

                        // Reset the flag after a delay to allow for any cascading updates
                        setTimeout(() => {
                            preserveRelativeZoom = false;
                        }, 200);
                    }, 0);
                }
            ],

            ready: [
                (u) => {
                    uZoomed = u;
                    const rangerData = u.data;
                    const rangerOpts: uPlot.Options = {
                        width: containerEl.clientWidth,
                        height: config.height || 100,
                        cursor: { drag: { setScale: false, x: false, y: false } },
                        legend: { show: false },
                        scales: { x: { time: u.scales.x.time } },
                        axes: [{ show: false }, { show: false }],
                        series: [{}, { stroke: 'red' }],
                        hooks: {
                            ready: [
                                (uRangerInstance) => {
                                    uRanger = uRangerInstance;
                                    const overlay = uRanger.root.querySelector('.u-over') as HTMLElement;
                                    if (overlay) {
                                        overlay.addEventListener(
                                            'dblclick',
                                            (e: MouseEvent) => {
                                                if ((e.target as HTMLElement).closest('.u-select')) return;
                                                e.preventDefault();
                                                e.stopPropagation();
                                                const rangerWidth = uRanger!.bbox.width / devicePixelRatio;
                                                uRanger!.setSelect(
                                                    {
                                                        left: 0,
                                                        top: 0,
                                                        width: rangerWidth,
                                                        height: uRanger!.bbox.height / devicePixelRatio
                                                    },
                                                    true
                                                );
                                            },
                                            true
                                        );
                                        overlay.addEventListener(
                                            'mousedown',
                                            (e: MouseEvent) => {
                                                if ((e.target as HTMLElement).closest('.u-select')) return;
                                                e.preventDefault();
                                                e.stopPropagation();
                                            },
                                            true
                                        );
                                    }
                                    const sel = uRanger.root.querySelector('.u-select') as HTMLElement;
                                    if (!sel) {
                                        return;
                                    }

                                    const dataRange = uRanger.data[0] as number[];

                                    if (dataRange && dataRange.length > 0) {
                                        const minVal = dataRange[0];
                                        const maxVal = dataRange[dataRange.length - 1];
                                        const rangeSize = maxVal - minVal;

                                        if (rangeSize > 0 && Number.isFinite(minVal) && Number.isFinite(maxVal)) {
                                            const selectionStart = minVal + rangeSize * 0.1;
                                            const selectionEnd = maxVal - rangeSize * 0.1;
                                            const left = Math.round(uRanger.valToPos(selectionStart, 'x'));
                                            const right = Math.round(uRanger.valToPos(selectionEnd, 'x'));
                                            const heightPx = uRanger.bbox.height / devicePixelRatio;

                                            uRanger.setSelect({ left, top: 0, width: right - left, height: heightPx }, false);

                                            // Initialize the relative zoom state
                                            const rangerWidth = uRanger.bbox.width / devicePixelRatio;
                                            if (rangerWidth > 0) {
                                                const leftRatio = left / rangerWidth;
                                                const rightRatio = right / rangerWidth;
                                                relativeZoomState = { leftRatio, rightRatio };
                                            }
                                        } else {
                                            const rangerWidth = uRanger.bbox.width / devicePixelRatio;
                                            uRanger.setSelect(
                                                { left: 0, top: 0, width: rangerWidth, height: uRanger.bbox.height / devicePixelRatio },
                                                false
                                            );
                                            relativeZoomState = { leftRatio: 0, rightRatio: 1 };
                                        }
                                    } else {
                                        // Check if config.init values are valid
                                        if (
                                            config.init &&
                                            config.init.length === 2 &&
                                            Number.isFinite(config.init[0]) &&
                                            Number.isFinite(config.init[1]) &&
                                            config.init[0] !== config.init[1]
                                        ) {
                                            const left = Math.round(uRanger.valToPos(config.init[0], 'x'));
                                            const right = Math.round(uRanger.valToPos(config.init[1], 'x'));
                                            const heightPx = uRanger.bbox.height / devicePixelRatio;

                                            uRanger.setSelect({ left, top: 0, width: right - left, height: heightPx }, false);

                                            // Initialize the relative zoom state
                                            const rangerWidth = uRanger.bbox.width / devicePixelRatio;
                                            if (rangerWidth > 0) {
                                                const leftRatio = left / rangerWidth;
                                                const rightRatio = right / rangerWidth;
                                                relativeZoomState = { leftRatio, rightRatio };
                                            }
                                        } else {
                                            const rangerWidth = uRanger.bbox.width / devicePixelRatio;
                                            uRanger.setSelect(
                                                {
                                                    left: 0,
                                                    top: 0,
                                                    width: rangerWidth,
                                                    height: uRanger.bbox.height / devicePixelRatio
                                                },
                                                false
                                            );
                                            relativeZoomState = { leftRatio: 0, rightRatio: 1 };
                                        }
                                    }
                                    const makeGrip = (cls: string, mode: 'resize-l' | 'resize-r' | 'move') => {
                                        const div = document.createElement('div');
                                        div.classList.add(cls);
                                        sel.appendChild(div);
                                        div.addEventListener('mousedown', (e) => {
                                            e.stopPropagation();
                                            bindMove(e, mode);
                                        });
                                    };
                                    makeGrip('u-grip-sel', 'move');
                                    makeGrip('u-grip-l', 'resize-l');
                                    makeGrip('u-grip-r', 'resize-r');

                                    // Ensure the main chart is properly scaled after initialization
                                    setTimeout(() => {
                                        if (uRanger && uZoomed && uRanger.select) {
                                            const { left, width } = uRanger.select;
                                            if (width > 0) {
                                                const min = uRanger.posToVal(left, 'x');
                                                const max = uRanger.posToVal(left + width, 'x');
                                                uZoomed.setScale('x', { min, max });
                                            }
                                        }
                                    }, 100); // Small delay to ensure everything is ready
                                }
                            ],
                            setSelect: [
                                () => {
                                    if (!uZoomed || !uRanger) return;
                                    const { left, width } = uRanger.select;
                                    if (width <= 0) return;

                                    // Update relative zoom state when user manually adjusts slider
                                    const rangerWidth = uRanger.bbox.width / devicePixelRatio;
                                    if (rangerWidth > 0) {
                                        const leftRatio = left / rangerWidth;
                                        const rightRatio = (left + width) / rangerWidth;
                                        relativeZoomState = { leftRatio, rightRatio };
                                    }

                                    const min = uRanger.posToVal(left, 'x');
                                    const max = uRanger.posToVal(left + width, 'x');
                                    uZoomed.setScale('x', { min, max });
                                }
                            ]
                        }
                    };
                    const debounce = (fn: (...args: any[]) => void) => {
                        let raf: number;
                        return (...args: any[]) => {
                            if (raf) return;
                            raf = requestAnimationFrame(() => {
                                fn(...args);
                                raf = 0;
                            });
                        };
                    };
                    function bindMove(downEv: MouseEvent, mode: 'move' | 'resize-l' | 'resize-r') {
                        if (!uRanger) return;
                        const startX = downEv.clientX;
                        const { left: baseLeft, width: baseWidth } = uRanger.select;
                        const plotWidth = uRanger.bbox.width / devicePixelRatio;
                        const onMove = debounce((moveEv: MouseEvent) => {
                            const dx = moveEv.clientX - startX;
                            let newLeft: number, newWidth: number;
                            if (mode === 'move') {
                                newLeft = baseLeft + dx;
                                newWidth = baseWidth;
                                if (newLeft < 0) {
                                    newWidth = baseWidth + newLeft;
                                    newLeft = 0;
                                }
                                if (newLeft + newWidth > plotWidth) {
                                    newWidth = plotWidth - newLeft;
                                }
                            } else if (mode === 'resize-l') {
                                newLeft = baseLeft + dx;
                                newWidth = baseWidth - dx;
                                if (newLeft < 0) {
                                    newLeft = 0;
                                    newWidth = baseLeft + baseWidth;
                                }
                                if (newWidth < 10) {
                                    newWidth = 10;
                                    newLeft = baseLeft + baseWidth - 10;
                                }
                            } else {
                                newLeft = baseLeft;
                                newWidth = baseWidth + dx;
                                if (newLeft + newWidth > plotWidth) {
                                    newWidth = plotWidth - newLeft;
                                }
                                if (newWidth < 10) {
                                    newWidth = 10;
                                }
                            }
                            if (uRanger) {
                                const currentSelect = uRanger.select;
                                if (Math.abs(currentSelect.left - newLeft) > 1 || Math.abs(currentSelect.width - newWidth) > 1) {
                                    uRanger.setSelect(
                                        {
                                            left: newLeft,
                                            top: 0,
                                            width: newWidth,
                                            height: uRanger.bbox.height / devicePixelRatio
                                        },
                                        false
                                    );
                                }
                            }
                        });
                        function onMouseUp() {
                            document.removeEventListener('mousemove', onMove);
                            document.removeEventListener('mouseup', onMouseUp);
                            if (uRanger) {
                                const currentSelect = uRanger.select;
                                uRanger.setSelect(
                                    {
                                        left: currentSelect.left,
                                        top: currentSelect.top,
                                        width: currentSelect.width,
                                        height: currentSelect.height
                                    },
                                    true
                                );
                            }
                        }
                        document.addEventListener('mousemove', onMove);
                        document.addEventListener('mouseup', onMouseUp);
                        downEv.preventDefault();
                    }
                    uRanger = new uPlot(rangerOpts, rangerData as any, containerEl);
                }
            ]
        }
    };
}
