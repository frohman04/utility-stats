#!/usr/bin/env python

import argparse
import csv
import datetime
import logging
import shutil

import plotly.graph_objs as go
import plotly.offline as pl

import tempdata


class Measurement(object):
    """A single meter reading."""

    def __init__(self, date, amount, units):
        """Create a new Measurement.

        Args:
            date (datetime.date): the date of the meter reading
            amount (float): the amount of resources used since the last meter
                    reading
            units (str): the units that the measurement is in
        """

        self._date = date
        self._amount = amount
        self._units = units

    def get_date(self):
        """Get the date of the meter reading."""

        return self._date

    def get_amount(self):
        """Get the amount of resources used since the last meter reading."""

        return self._amount

    def get_units(self):
        """Return:
            (str): The units that the measurement is in
        """

        return self._units

    def __repr__(self):
        return 'Measurement(%s, %s, %s)' % (
                self._date,
                self._amount,
                self._units)

    def __str__(self):
        return '%s: %s%s' % (self._date, self._amount, self._units)


def read_file(file_name, units):
    """Read a CSV meter reading file into memory.

    Args:
        file_name (path str): the path to the file to read
        units (str): the units for the amounts read from the file

    Return:
        ([Measurement]): the data from the file
    """

    LOGGER.info('Reading file: %s' % file_name)
    measurements = []
    with open(file_name, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            measurements += [Measurement(
                    datetime.datetime.strptime(row[0], '%Y-%m-%d').date(),
                    int(row[1]),
                    units)]
            LOGGER.debug('\tRead: %s' % measurements[-1])
    return measurements


def get_plot_data(data):
    """Get the plottable data series for a given measurement dataset.

    Args:
        data ([Measurement]): the measurements to get plot data for

    Return:
        ([float], [float]): tuple of arrays of X data and Y data
    """

    x_data = []
    y_data = []
    for i in range(1, len(data)):
        num_days = (data[i].get_date() - data[i - 1].get_date()).days
        avg_usage = data[i].get_amount() / num_days
        x_data += [data[i].get_date()]
        y_data += [avg_usage]
    return (x_data, y_data)


def get_temp_data(util_data, temp_mgr, measurement):
    """Get the temperature data to plot for a set of data for a utility.

    Args:
        util_data ([Measurement]): the utility to get temperatures for
        temp_mgr (tempdata.TempDataManager): the temperature data manager to
                query for temperature data
        measurement (str): one of 'min', 'mean', 'max'

    Return:
        ([float], [float]): tuple of arrays of X data and Y data
    """

    if measurement == 'min':
        func = temp_mgr.get_avg_min_temp
    elif measurement == 'mean':
        func = temp_mgr.get_avg_mean_temp
    elif measurement == 'max':
        func = temp_mgr.get_avg_max_temp
    else:
        raise('Unknown measurement type: %s; expected one of min, mean, max' %
              measurement)

    x_data = []
    y_data = []
    for i in range(1, len(util_data)):
        avg_temp = func(
                util_data[i - 1].get_date(),
                util_data[i].get_date())
        x_data += [util_data[i].get_date()]
        y_data += [avg_temp]
    return (x_data, y_data)


def main(gas_file, elec_file):
    """Main program method.

    Args:
        gas_file (path str): the CSV for gas meter readings
        elec_file (path str): the CSV file for electric meter readings
    """

    gas_data = read_file(gas_file, 'CCF')
    elec_data = read_file(elec_file, 'kWh')

    gas_plot_data = get_plot_data(gas_data)
    elec_plot_data = get_plot_data(elec_data)

    temp_mgr = tempdata.TempDataManager()
    gas_temp_plot_data = get_temp_data(gas_data, temp_mgr, 'min')
    elec_temp_plot_data = get_temp_data(elec_data, temp_mgr, 'max')

    layout = go.Layout(
        title='Gas Usage',
        xaxis=dict(
            title='Meaurement Date'
        ),
        yaxis=dict(
            title='Avg Low Temp (F)'
        ),
        yaxis2=dict(
            title='CCF used / day',
            overlaying='y',
            side='right'
        )
    )
    fig = go.Figure(
        data=[
            go.Scatter(
                x=gas_temp_plot_data[0],
                y=gas_temp_plot_data[1],
                mode='lines'
            ),
            go.Scatter(
                x=gas_plot_data[0],
                y=gas_plot_data[1],
                mode='lines',
                yaxis='y2'
            )
        ],
        layout=layout)
    pl.plot(fig)
    shutil.move('temp-plot.html', 'gas.html')

    layout = go.Layout(
        title='Electricity Usage',
        xaxis=dict(
            title='Meaurement Date'
        ),
        yaxis=dict(
            title='Avg High Temp (F)'
        ),
        yaxis2=dict(
            title='kWh used / day',
            overlaying='y',
            side='right'
        )
    )
    fig = go.Figure(
        data=[
            go.Scatter(
                x=elec_temp_plot_data[0],
                y=elec_temp_plot_data[1],
                mode='lines'
            ),
            go.Scatter(
                x=elec_plot_data[0],
                y=elec_plot_data[1],
                mode='lines',
                yaxis='y2'
            )
        ],
        layout=layout)
    pl.plot(fig)
    shutil.move('temp-plot.html', 'electric.html')


def parse_args():
    """Parse the command line arguments.

    Return:
        ({str: str}): the arguments
    """

    parser = argparse.ArgumentParser(
            description='Generate neat charts from utility usage data')
    parser.add_argument(
            'gas',
            metavar='GAS_FILE',
            type=str,
            help='the CSV file that contains the gas meter readings')
    parser.add_argument(
            'electric',
            metavar='ELEC_FILE',
            type=str,
            help='the CSV file that contains the electric meter readings')
    args = parser.parse_args()
    return {
        'gas': args.gas,
        'elec': args.electric
    }

logging.basicConfig(level=logging.INFO)
LOGGER = logging.getLogger(name=__name__)

if __name__ == '__main__':
    args = parse_args()
    main(args['gas'], args['elec'])
