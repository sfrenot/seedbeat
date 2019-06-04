// Licensed to Elasticsearch B.V. under one or more contributor
// license agreements. See the NOTICE file distributed with
// this work for additional information regarding copyright
// ownership. Elasticsearch B.V. licenses this file to you under
// the Apache License, Version 2.0 (the "License"); you may
// not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

// Code generated by beats/dev-tools/cmd/asset/asset.go - DO NOT EDIT.

package include

import (
	"github.com/elastic/beats/libbeat/asset"
)

func init() {
	if err := asset.SetFields("seedbeat", "fields.yml", asset.BeatFieldsPri, AssetFieldsYml); err != nil {
		panic(err)
	}
}

// AssetFieldsYml returns asset data.
// This is the base64 encoded gzipped contents of fields.yml.
func AssetFieldsYml() string {
	return "eJxUzMFuskAUxfE9T3G+uFX43LJoQpQ0Ji2aqul6YI5wW5yhMxcb3r7RsGh39/6S81/gvXirdtXzP2w9nFfQikI7ibhIT1gJbLSflhDFt4lo6RiM0qKeoB1Rbo4Ygv9go8tkgdpEWnj38BtDFO+wTv+n6zRZ4NDTROImURSd6hDzLGtFu7FOG3/N2Juo0mRsItQjjm3LqGg641o+6J69CHsb0yRZ4ZNTjkjamkYTQEV7/hHL2AQZVLzLE8zb+wWs4MyVOQYyPADQaWAOGeY38GuUQJtDw8gZfxfxNCNweCmLY4nzYVucSmz3m/NrWZ2K025fJT8BAAD//2yacAo="
}
