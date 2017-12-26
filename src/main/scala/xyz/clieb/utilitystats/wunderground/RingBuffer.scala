package xyz.clieb.utilitystats.wunderground

import com.typesafe.scalalogging.LazyLogging

import scala.collection.mutable

private[wunderground] class RingBuffer[T](size: Int) extends LazyLogging {
  private val buffer = mutable.IndexedSeq.fill[T](size)(null.asInstanceOf[T])
  private var curr = 0
  private var next = 1

  def add(value: T): Unit = {
    curr = next
    next = (curr + 1) % buffer.length
    buffer(curr) = value
  }

  def peekTail: T = buffer(next)
}
