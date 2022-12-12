import type { PhotonImage } from '..'
import { createScheduler, createWorker, RecognizeResult, Scheduler, Worker } from 'tesseract.js'

export type TesseractOptions = {
  numWorkers: number,
  langs: string
  workerOptions?: Partial<Tesseract.WorkerOptions>,
  workerParams?: Partial<Tesseract.WorkerParams>,
}

export const TesseractDefaults: TesseractOptions = {
  numWorkers: 3,
  langs: 'eng',
  workerOptions: {
    errorHandler: (error: any) => {
      console.error('[tesseract] Encountered an error inside a Tesseract worker')
      console.error(error)
    }
  }
}

export async function initTesseractScheduler(options: Partial<TesseractOptions> = {}): Promise<Scheduler> {
  const { workerOptions, workerParams, numWorkers, langs } = Object.assign(options, TesseractDefaults)
  const scheduler = createScheduler()

  for (let i = 0; i < numWorkers; i++) {
    try {
      const worker = await initTesseractWorker(workerOptions, workerParams, langs)
      scheduler.addWorker(worker)
    } catch (e) {
      console.error(e)
    }
  }

  return scheduler
}

export async function initTesseractWorker(
  workerOptions: Partial<Tesseract.WorkerOptions>,
  workerParams: Partial<Tesseract.WorkerParams>,
  langs: string,
): Promise<Worker> {
  const worker = createWorker(workerOptions)
  await worker.load()
  await worker.loadLanguage(langs)
  await worker.initialize(langs)
  await worker.setParameters(workerParams)
  return worker
}

export async function recognizeText(image: PhotonImage, scheduler: Scheduler): Promise<string> {
  const imageData = image.get_image_data()
  const result = await scheduler.addJob('recognize', imageData) as RecognizeResult
  const text = result.data.text
  return text
}
