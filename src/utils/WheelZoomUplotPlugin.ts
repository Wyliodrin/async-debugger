import type { Ref } from 'vue'
import uPlot from 'uplot'

export function wheelZoomPlugin(opts: any) {
  let factor = opts.factor || 0.75;

  let xMin, xMax, yMin, yMax, xRange, yRange;

  function clamp(nRange, nMin, nMax, fRange, fMin, fMax) {
    if (nRange > fRange) {
      nMin = fMin;
      nMax = fMax;
    }
    else if (nMin < fMin) {
      nMin = fMin;
      nMax = fMin + nRange;
    }
    else if (nMax > fMax) {
      nMax = fMax;
      nMin = fMax - nRange;
    }

    return [nMin, nMax];
  }

  return {
    hooks: {
      ready: u => {
        xMin = u.scales.x.min;
        xMax = u.scales.x.max;
        yMin = u.scales.y.min;
        yMax = u.scales.y.max;

        xRange = xMax - xMin;
        yRange = yMax - yMin;

        let over = u.over;
        let rect = over.getBoundingClientRect();

        // wheel drag pan
        over.addEventListener("mousedown", e => {
          if (e.button == 1) {
            //	plot.style.cursor = "move";
            e.preventDefault();

            let left0 = e.clientX;
            //	let top0 = e.clientY;

            let scXMin0 = u.scales.x.min;
            let scXMax0 = u.scales.x.max;

            let xUnitsPerPx = u.posToVal(1, 'x') - u.posToVal(0, 'x');

            function onmove(e) {
              e.preventDefault();

              let left1 = e.clientX;
              //	let top1 = e.clientY;

              let dx = xUnitsPerPx * (left1 - left0);

              u.setScale('x', {
                min: scXMin0 - dx,
                max: scXMax0 - dx,
              });
            }

            function onup(e) {
              document.removeEventListener("mousemove", onmove);
              document.removeEventListener("mouseup", onup);
            }

            document.addEventListener("mousemove", onmove);
            document.addEventListener("mouseup", onup);
          }
        });

        // wheel scroll zoom
        over.addEventListener("wheel", e => {
          e.preventDefault();

          let { left, top } = u.cursor;

          let leftPct = left / rect.width;
          let btmPct = 1 - top / rect.height;
          let xVal = u.posToVal(left, "x");
          let yVal = u.posToVal(top, "y");
          // Update ranges to current data (since data changes constantly)
          let currentXMin = u.scales.x.min;
          let currentXMax = u.scales.x.max;
          let currentYMin = u.scales.y.min;
          let currentYMax = u.scales.y.max;
          let currentXRange = currentXMax - currentXMin;
          let currentYRange = currentYMax - currentYMin;

          let oxRange = currentXRange;
          let oyRange = currentYRange;

          let nxRange = e.deltaY < 0 ? oxRange * factor : oxRange / factor;
          let nxMin = xVal - leftPct * nxRange;
          let nxMax = nxMin + nxRange;

          // Only clamp when zooming in (nxRange < currentXRange), allow zooming out beyond current range
          if (nxRange < currentXRange) {
            [nxMin, nxMax] = clamp(nxRange, nxMin, nxMax, currentXRange, currentXMin, currentXMax);
          }

          let nyRange = e.deltaY < 0 ? oyRange * factor : oyRange / factor;
          let nyMin = yVal - btmPct * nyRange;
          let nyMax = nyMin + nyRange;

          // Only clamp when zooming in (nyRange < currentYRange), allow zooming out beyond current range
          if (nyRange < currentYRange) {
            [nyMin, nyMax] = clamp(nyRange, nyMin, nyMax, currentYRange, currentYMin, currentYMax);
          }

          u.batch(() => {
            u.setScale("x", {
              min: nxMin,
              max: nxMax,
            });

            u.setScale("y", {
              min: nyMin,
              max: nyMax,
            });
          });
        });
      }
    }
  };
}