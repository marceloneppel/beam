/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
/*
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// beam-playground:
//   name: read-query
//   description: BigQuery read query example.
//   multifile: false
//   context_line: 56
//   categories:
//     - Quickstart
//   complexity: ADVANCED
//   tags:
//     - hellobeam

import com.google.api.services.bigquery.model.TableRow;
import org.apache.beam.sdk.Pipeline;
import org.apache.beam.sdk.io.gcp.bigquery.BigQueryIO;
import org.apache.beam.sdk.io.gcp.bigquery.BigQueryIO.TypedRead;
import org.apache.beam.sdk.options.PipelineOptions;
import org.apache.beam.sdk.options.PipelineOptionsFactory;
import org.apache.beam.sdk.transforms.DoFn;
import org.apache.beam.sdk.transforms.ParDo;
import org.apache.beam.sdk.values.PCollection;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;


public class Task {
    private static final Logger LOG = LoggerFactory.getLogger(Task.class);

    private static final String WEATHER_SAMPLES_QUERY =
            "select * from `clouddataflow-readonly.samples.weather_stations`";

    public static void applyBigQueryTornadoes(Pipeline p) {
        /*TypedRead<TableRow> bigqueryIO =
                BigQueryIO.readTableRows()
                        .fromQuery(WEATHER_SAMPLES_QUERY)
                        .usingStandardSql();


        PCollection<TableRow> rowsFromBigQuery = p.apply(bigqueryIO);

        rowsFromBigQuery
                .apply(ParDo.of(new LogOutput<>("Result: ")));*/
    }

    public static void runBigQueryTornadoes(PipelineOptions options) {
        Pipeline p = Pipeline.create(options);
        applyBigQueryTornadoes(p);
        p.run().waitUntilFinish();
    }

    public static void main(String[] args) {
        PipelineOptions options =
                PipelineOptionsFactory.fromArgs(args).withValidation().as(PipelineOptions.class);
        runBigQueryTornadoes(options);
    }

    static class LogOutput<T> extends DoFn<T, T> {
        private final String prefix;

        LogOutput(String prefix) {
            this.prefix = prefix;
        }

        @ProcessElement
        public void processElement(ProcessContext c) {
            LOG.info(prefix + c.element());
            c.output(c.element());
        }
    }
}
